use crate::app::Keyforge;
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, IndexPath,
    input::{InputEvent, InputState},
    select::{SelectDelegate, SelectEvent, SelectItem, SelectState},
};
use keyforge_core::{
    Command,
    bindings::{self, ParameterKind},
    editing::BindingEdit,
};

#[derive(Clone)]
pub(crate) struct BindingChoice {
    pub value: String,
    pub description: String,
    pub label: Option<String>,
}
impl BindingChoice {
    pub fn new(value: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            description: description.into(),
            label: None,
        }
    }
}
impl BindingChoice {
    pub(crate) fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }
}
impl SelectItem for BindingChoice {
    type Value = String;
    fn title(&self) -> SharedString {
        format!("{} · {}", self.value, self.description).into()
    }
    fn value(&self) -> &String {
        &self.value
    }
    fn display_title(&self) -> Option<AnyElement> {
        Some(
            div()
                .truncate()
                .child(if let Some(label) = &self.label {
                    label.clone()
                } else if self.value.is_empty() {
                    self.description.clone()
                } else {
                    self.value.clone()
                })
                .into_any_element(),
        )
    }
    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().flex_shrink_0().child(self.value.clone()))
            .child(
                div()
                    .truncate()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child(self.description.clone()),
            )
    }
}
pub(crate) struct BindingOptions {
    items: Vec<BindingChoice>,
    matches: Vec<BindingChoice>,
}
impl BindingOptions {
    pub(crate) fn new(items: Vec<BindingChoice>) -> Self {
        Self {
            matches: items.clone(),
            items,
        }
    }
}
impl SelectDelegate for BindingOptions {
    type Item = BindingChoice;
    fn items_count(&self, _: usize) -> usize {
        self.matches.len()
    }
    fn item(&self, index: IndexPath) -> Option<&Self::Item> {
        self.matches.get(index.row)
    }
    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SelectItem<Value = V>,
        V: PartialEq,
    {
        self.matches
            .iter()
            .position(|item| item.value() == value)
            .map(|row| IndexPath::default().row(row))
    }
    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<SelectState<Self>>,
    ) -> Task<()> {
        self.matches = crate::parameter_picker::search(&self.items, query);
        Task::ready(())
    }
}
pub(crate) type BindingChoices = SelectState<BindingOptions>;
pub(crate) fn new_choices(
    window: &mut Window,
    cx: &mut Context<Keyforge>,
) -> Entity<BindingChoices> {
    cx.new(|cx| {
        SelectState::new(
            BindingOptions::new(Vec::<BindingChoice>::new()),
            None,
            window,
            cx,
        )
        .searchable(true)
    })
}

impl Keyforge {
    pub(crate) fn install_binding_controls(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._subscriptions.push(cx.subscribe_in(
            &self.behavior_select,
            window,
            |this, _, event, window, cx| {
                if let SelectEvent::Confirm(Some(reference)) = event {
                    if reference.is_empty() {
                        this.binding_advanced = true;
                        cx.notify();
                    } else if this
                        .core
                        .assignment_form(&this.binding_target)
                        .is_some_and(|f| f.reference != *reference)
                    {
                        let reference = reference.clone();
                        cx.defer_in(window, move |this, window, cx| {
                            this.binding_advanced = false;
                            this.expanded_action = None;
                            this.picker_context = None;
                            this.command(
                                Command::EditBinding {
                                    target: this.binding_target.clone(),
                                    edit: BindingEdit::Behavior(reference),
                                },
                                window,
                                cx,
                            );
                        });
                    }
                }
            },
        ));
        self._subscriptions.push(cx.subscribe_in(
            &self.raw_binding_input,
            window,
            |this, input, event, window, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = input.read(cx).value().to_string();
                    if this
                        .core
                        .assignment_source(&this.binding_target)
                        .is_some_and(|source| source != value)
                    {
                        this.command(
                            Command::EditBinding {
                                target: this.binding_target.clone(),
                                edit: BindingEdit::Expression(value),
                            },
                            window,
                            cx,
                        );
                    }
                }
            },
        ));
    }
    pub(crate) fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.core.project.zmk.as_ref().map(|d| &d.source);
        if self.binding_catalog.is_empty()
            || self.binding_catalog_source.as_ref() != source
            || self.binding_catalog_target.as_ref() != Some(&self.binding_target)
        {
            self.binding_catalog = self.core.assignment_catalog(&self.binding_target);
            self.hold_tap_behaviors = self
                .core
                .project
                .zmk
                .as_ref()
                .and_then(|document| document.hold_tap_behaviors().ok())
                .unwrap_or_default();
            self.binding_catalog_target = Some(self.binding_target.clone());
            self.binding_catalog_source = source.cloned();
            let mut choices = self
                .binding_catalog
                .iter()
                .map(|spec| {
                    BindingChoice::new(&spec.reference, &spec.name)
                        .with_label(format!("{} ({})", spec.name, spec.reference))
                })
                .collect::<Vec<_>>();
            choices.push(BindingChoice::new("", "Custom / module binding…"));
            self.behavior_select.update(cx, |state, cx| {
                state.set_items(BindingOptions::new(choices), window, cx)
            });
        }
        self.binding_errors = self.core.assignment_errors(&self.binding_target);
        self.binding_preview = None;
        let Some(form) = self.core.assignment_form(&self.binding_target) else {
            return;
        };
        self.binding_preview = self
            .core
            .project
            .zmk
            .as_ref()
            .and_then(|document| bindings::definition_preview(document, &form.reference));
        let spec = self
            .binding_catalog
            .iter()
            .find(|s| s.reference == form.reference);
        let selected = if form.reference.is_empty() {
            String::new()
        } else {
            form.reference.clone()
        };
        let mut choices = self
            .binding_catalog
            .iter()
            .map(|spec| {
                BindingChoice::new(&spec.reference, &spec.name)
                    .with_label(format!("{} ({})", spec.name, spec.reference))
            })
            .collect::<Vec<_>>();
        if spec.is_none() && !selected.is_empty() {
            choices.push(
                BindingChoice::new(&selected, "Unrecognized behavior")
                    .with_label(format!("Unrecognized ({selected})")),
            );
        }
        choices.push(BindingChoice::new("", "Custom / module binding…"));
        self.behavior_select.update(cx, |state, cx| {
            state.set_items(BindingOptions::new(choices), window, cx);
            state.set_selected_value(&selected, window, cx);
        });
        let fields = spec.map(|s| s.fields(&form.arguments)).unwrap_or_default();
        let raw = self
            .core
            .assignment_source(&self.binding_target)
            .unwrap_or_default();
        if self.raw_binding_input.read(cx).value().as_ref() != raw {
            self.raw_binding_input
                .update(cx, |state, cx| state.set_value(raw, window, cx));
        }
        while self.inputs.len() < fields.len() {
            let index = self.inputs.len();
            let input = cx.new(|cx| InputState::new(window, cx));
            self._subscriptions.push(cx.subscribe_in(
                &input,
                window,
                move |this, input, event, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        let value = input.read(cx).value().to_string();
                        if this
                            .core
                            .assignment_form(&this.binding_target)
                            .is_some_and(|f| {
                                f.arguments.get(index).map(String::as_str).unwrap_or("") != value
                            })
                        {
                            this.command(
                                Command::EditBinding {
                                    target: this.binding_target.clone(),
                                    edit: BindingEdit::Argument {
                                        index,
                                        value,
                                        selected: false,
                                    },
                                },
                                window,
                                cx,
                            );
                        }
                    }
                },
            ));
            self.inputs.push(input);
        }
        for (index, field) in fields.iter().enumerate() {
            let value = form.arguments.get(index).cloned().unwrap_or_default();
            if self.inputs[index].read(cx).value().as_ref() != value {
                self.inputs[index]
                    .update(cx, |state, cx| state.set_value(value.clone(), window, cx));
            }
            let selected = if field.kind == ParameterKind::KeyCode {
                bindings::modifiers(&value).0
            } else {
                value.clone()
            };
            let mut choices = match &field.kind {
                ParameterKind::KeyCode => self.binding_keycodes.clone(),
                ParameterKind::Layer => self
                    .core
                    .project
                    .keymap
                    .layers
                    .iter()
                    .enumerate()
                    .map(|(i, layer)| BindingChoice::new(i.to_string(), &layer.name))
                    .collect(),
                ParameterKind::Choice(values) => values
                    .iter()
                    .map(|value| BindingChoice::new(value, ""))
                    .collect(),
                ParameterKind::Expression => vec![],
            };
            if !choices.iter().any(|item| item.value == selected) && !selected.is_empty() {
                choices.insert(0, BindingChoice::new(&selected, "Current value"));
            }
            if self.expanded_action == Some(index) && field.kind != ParameterKind::Expression {
                let context = (self.binding_target.clone(), index);
                if self.picker_context.as_ref() != Some(&context) || self.parameter_picker.is_none()
                {
                    let owner = cx.entity().downgrade();
                    let target = self.binding_target.clone();
                    let picker = cx.new(|cx| {
                        gpui_component::list::ListState::new(
                            crate::parameter_picker::ParameterPicker::new(
                                choices, target, index, owner,
                            ),
                            window,
                            cx,
                        )
                        .searchable(true)
                    });
                    picker.update(cx, |picker, cx| picker.focus(window, cx));
                    self.parameter_picker = Some(picker);
                    self.picker_context = Some(context);
                } else if let Some(picker) = &self.parameter_picker {
                    picker.update(cx, |picker, cx| {
                        picker.delegate_mut().set_items(choices);
                        cx.notify();
                    });
                }
            }
        }
    }
}
