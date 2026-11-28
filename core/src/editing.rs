//! Contextual binding edits share one parser, metadata model and transaction path.
use crate::{
    Application, BindingDraft, Command, Error, Result, Severity,
    bindings::{self, Form, ParameterKind},
    editor, zmk,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingTarget {
    Key,
    Field { field: String, binding: usize },
    Child { field: String, binding: usize },
    Sensor,
}
#[derive(Debug, Clone)]
pub enum BindingEdit {
    Behavior(String),
    Argument {
        index: usize,
        value: String,
        selected: bool,
    },
    Modifier {
        index: usize,
        modifier: String,
    },
    Expression(String),
}
#[derive(Debug, Clone)]
pub(crate) struct NestedDraft {
    pub target: BindingTarget,
    pub draft: BindingDraft,
}

impl Application {
    pub fn assignment_source(&self, target: &BindingTarget) -> Option<String> {
        if *target == BindingTarget::Key {
            return self.draft.as_ref().map(BindingDraft::source);
        }
        if let Some(active) = &self.nested_draft
            && active.target == *target
        {
            return Some(active.draft.source());
        }
        let draft = self.editor.as_ref()?;
        match target {
            BindingTarget::Field { field, binding } | BindingTarget::Child { field, binding } => {
                let value = draft.get(field);
                let span = zmk::binding_spans(value).ok()?.get(*binding)?.clone();
                Some(value[span].to_owned())
            }
            BindingTarget::Sensor => Some(draft.get("sensor-binding").to_owned()),
            BindingTarget::Key => None,
        }
    }
    pub fn assignment_form(&self, target: &BindingTarget) -> Option<Form> {
        let draft = if *target == BindingTarget::Key {
            self.draft.as_ref()
        } else {
            self.nested_draft
                .as_ref()
                .filter(|d| d.target == *target)
                .map(|d| &d.draft)
        };
        if let Some(draft) = draft
            && let Some(reference) = &draft.reference
        {
            return Some(Form {
                reference: reference.clone(),
                arguments: draft.parameters.clone(),
            });
        }
        self.assignment_source(target).map(|source| {
            let (reference, arguments) = bindings::editing_parts(&source);
            Form {
                reference,
                arguments,
            }
        })
    }
    pub fn assignment_catalog(&self, target: &BindingTarget) -> Vec<bindings::BehaviorSpec> {
        if *target == BindingTarget::Sensor {
            bindings::sensor_catalog(self.project.zmk.as_ref())
        } else if matches!(target, BindingTarget::Field { field, .. } if field.starts_with("step-bindings-"))
        {
            bindings::macro_catalog(self.project.zmk.as_ref())
        } else {
            bindings::catalog(self.project.zmk.as_ref())
        }
    }
    pub fn assignment_errors(&self, target: &BindingTarget) -> Vec<String> {
        let Some(source) = self.assignment_source(target) else {
            return vec![];
        };
        if let Err(error) = zmk::binding(&source) {
            return vec![error.to_string()];
        }
        if matches!(target, BindingTarget::Child { .. }) {
            return self
                .assignment_form(target)
                .filter(|form| !form.arguments.is_empty())
                .map(|_| vec!["Child behavior arguments belong to key assignments".into()])
                .unwrap_or_default();
        }
        self.assignment_form(target)
            .map(|form| {
                bindings::validate_form(
                    &form,
                    &self.assignment_catalog(target),
                    self.project.keymap.layers.len(),
                )
            })
            .unwrap_or_default()
    }
    pub fn nested_draft_invalid(&self) -> bool {
        self.nested_draft
            .as_ref()
            .is_some_and(|draft| !self.assignment_errors(&draft.target).is_empty())
    }
    pub(crate) fn edit_assignment(
        &mut self,
        target: BindingTarget,
        edit: BindingEdit,
    ) -> Result<()> {
        if self
            .nested_draft
            .as_ref()
            .is_some_and(|draft| draft.target != target)
            && self.nested_draft_invalid()
        {
            return Err(Error::InvalidDocument(
                "Finish or discard the current invalid action before editing another action".into(),
            ));
        }
        let catalog = self.assignment_catalog(&target);
        let mut form = self
            .assignment_form(&target)
            .ok_or(Error::InvalidSelection)?;
        let draft = match edit {
            BindingEdit::Expression(source) => BindingDraft {
                behavior: crate::Behavior::Custom,
                parameters: vec![source, String::new()],
                reference: None,
            },
            BindingEdit::Behavior(reference) => {
                let spec = catalog
                    .iter()
                    .find(|spec| spec.reference == reference)
                    .ok_or(Error::InvalidSelection)?;
                let previous = catalog.iter().find(|spec| spec.reference == form.reference);
                let tap = previous
                    .and_then(|spec| {
                        spec.fields(&form.arguments)
                            .iter()
                            .rposition(|p| p.kind == ParameterKind::KeyCode)
                    })
                    .and_then(|index| form.arguments.get(index))
                    .cloned();
                form.reference = reference;
                form.arguments = spec.parameters.iter().map(|p| p.default.clone()).collect();
                if let Some(tap) = tap
                    && let Some(index) = spec
                        .parameters
                        .iter()
                        .rposition(|p| p.kind == ParameterKind::KeyCode)
                {
                    form.arguments[index] = tap;
                }
                form.arguments.truncate(spec.fields(&form.arguments).len());
                if matches!(target, BindingTarget::Child { .. }) {
                    form.arguments.clear();
                }
                bindings::draft(&form.reference, &form.arguments)
            }
            BindingEdit::Argument {
                index,
                value,
                selected,
            } => {
                let spec = catalog
                    .iter()
                    .find(|spec| spec.reference == form.reference)
                    .ok_or(Error::InvalidSelection)?;
                let fields = spec.fields(&form.arguments);
                let field = fields.get(index).ok_or(Error::InvalidSelection)?;
                form.arguments.resize(fields.len(), String::new());
                form.arguments[index] = if selected && field.kind == ParameterKind::KeyCode {
                    let modifiers = bindings::modifiers(&form.arguments[index])
                        .1
                        .into_iter()
                        .filter(|m| !bindings::intrinsic_modifiers(&value).contains(m))
                        .collect::<Vec<_>>();
                    bindings::with_modifiers(&value, &modifiers)
                } else {
                    value
                };
                let fields = spec.fields(&form.arguments);
                form.arguments.truncate(fields.len());
                while form.arguments.len() < fields.len() {
                    form.arguments
                        .push(fields[form.arguments.len()].default.clone());
                }
                bindings::draft(&form.reference, &form.arguments)
            }
            BindingEdit::Modifier { index, modifier } => {
                let spec = catalog
                    .iter()
                    .find(|spec| spec.reference == form.reference)
                    .ok_or(Error::InvalidSelection)?;
                if !spec
                    .fields(&form.arguments)
                    .get(index)
                    .is_some_and(|p| p.kind == ParameterKind::KeyCode)
                    || !bindings::MODIFIER_FUNCTIONS
                        .iter()
                        .any(|(m, _)| *m == modifier)
                {
                    return Err(Error::InvalidSelection);
                }
                let (base, mut modifiers) =
                    bindings::modifiers(form.arguments.get(index).ok_or(Error::InvalidSelection)?);
                if bindings::intrinsic_modifiers(&base).contains(&modifier) {
                    return Ok(());
                }
                if modifiers.contains(&modifier) {
                    modifiers.retain(|m| m != &modifier);
                } else {
                    modifiers.push(modifier);
                }
                form.arguments[index] = bindings::with_modifiers(&base, &modifiers);
                bindings::draft(&form.reference, &form.arguments)
            }
        };
        if target == BindingTarget::Key {
            self.draft = Some(draft);
        } else {
            self.nested_draft = Some(NestedDraft {
                target: target.clone(),
                draft,
            });
        }
        if !self.assignment_errors(&target).is_empty() {
            self.status = "Invalid draft retained; the workspace is unchanged".into();
            return Ok(());
        }
        if target == BindingTarget::Key {
            if self
                .draft_diagnostics()
                .iter()
                .any(|d| d.severity == Severity::Error)
            {
                return Ok(());
            }
            self.dispatch(Command::ApplyBinding)?;
            self.status = "Changes staged · Save and build to update firmware".into();
        } else {
            let source = self
                .assignment_source(&target)
                .ok_or(Error::InvalidSelection)?;
            let draft = self.editor.as_mut().ok_or(Error::InvalidSelection)?;
            match target {
                BindingTarget::Sensor => draft.set("sensor-binding", source)?,
                BindingTarget::Field { ref field, binding }
                | BindingTarget::Child { ref field, binding } => {
                    let value = draft.get(field);
                    let span = zmk::binding_spans(value)?
                        .get(binding)
                        .cloned()
                        .ok_or(Error::InvalidSelection)?;
                    let mut value = value.to_owned();
                    value.replace_range(span, &source);
                    draft.set(field, value.clone())?;
                    if field.starts_with("step-bindings-")
                        && zmk::binding_spans(&value)?.len() == 1
                        && value.trim_start().starts_with("&macro_")
                    {
                        draft.set(&field.replace("step-bindings-", "step-mode-"), "Raw".into())?;
                    }
                }
                BindingTarget::Key => unreachable!(),
            }
            self.nested_draft = None;
            self.stage_definition();
        }
        Ok(())
    }
    pub(crate) fn stage_definition(&mut self) {
        let Some(draft) = self.editor.as_ref().filter(|draft| draft.dirty()).cloned() else {
            return;
        };
        match editor::apply(&self.project, &draft) {
            Ok(document) => {
                let Ok(layers) = document.layers() else {
                    return;
                };
                let before = self.project.clone();
                self.project.keymap.layers = layers;
                self.project.zmk = Some(document);
                self.record_edit(before);
                let selected = editor::entries(&self.project, draft.section, self.active_layer.0)
                    .ok()
                    .and_then(|entries| {
                        entries
                            .into_iter()
                            .find(|entry| entry.name == draft.get("name").trim())
                            .map(|entry| entry.id)
                    })
                    .or(draft.node.clone());
                if let Some(current) = &mut self.editor {
                    current.node = selected;
                    for field in &mut current.fields {
                        field.initial = field.value.clone();
                    }
                    current.sequence_changed = false;
                    current.error = None;
                }
                self.validate();
                self.reset_draft();
                self.status = "Changes staged · Save and build to update firmware".into();
            }
            Err(error) => {
                if let Some(current) = &mut self.editor {
                    current.error = Some(error.to_string());
                }
                self.status =
                    "Complete or discard the invalid draft before leaving this editor".into();
            }
        }
    }
}
