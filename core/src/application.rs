use crate::{
    domain::*,
    error::{Error, Result},
    persistence::LocalStorage,
    services::{self, BuildClock, BuildEvent, StorageEvent, StorageRequest, StorageWorker},
    workspace::{Appearance, BottomTab, Pane, Workspace},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    Keymap,
    Combos,
    Macros,
    Behaviors,
    ConditionalLayers,
    Encoders,
    Configuration,
    Builds,
    Artifacts,
    Repository,
    Settings,
}
impl Destination {
    pub const ALL: [Self; 11] = [
        Self::Keymap,
        Self::Combos,
        Self::Macros,
        Self::Behaviors,
        Self::ConditionalLayers,
        Self::Encoders,
        Self::Configuration,
        Self::Builds,
        Self::Artifacts,
        Self::Repository,
        Self::Settings,
    ];
}
impl std::fmt::Display for Destination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Keymap => "Keymap",
            Self::Combos => "Combos",
            Self::Macros => "Macros",
            Self::Behaviors => "Behaviors",
            Self::ConditionalLayers => "Conditional layers",
            Self::Encoders => "Encoders",
            Self::Configuration => "Configuration",
            Self::Builds => "Builds",
            Self::Artifacts => "Artifacts",
            Self::Repository => "Repository",
            Self::Settings => "Settings",
        })
    }
}
#[derive(Debug, Clone)]
pub enum Command {
    ExportArtifact {
        build: u64,
        name: String,
        directory: std::path::PathBuf,
    },
    EditBinding {
        target: crate::editing::BindingTarget,
        edit: crate::editing::BindingEdit,
    },
    DiscardBindingDraft,
    OpenDefinition(String),
    ToggleEditorLayer {
        field: String,
        layer: LayerId,
    },
    CheckBuildEnvironment,
    PrepareBuildEnvironment,
    DismissBuildSetup,
    OpenSample,
    ConnectGitHub,
    FindConfigurations,
    BrowseRepository,
    BrowseBranch(String),
    InsertMacroStep(usize),
    SetMacroExpression {
        index: usize,
        source: String,
    },
    SetConnection {
        field: String,
        value: String,
    },
    OpenRepository,
    ReloadSource,
    OpenLocal(std::path::PathBuf),
    SaveLocalCopy {
        directory: std::path::PathBuf,
        build: bool,
    },
    SaveAndBuildOnGitHub,
    DiscardChanges,
    SelectEditor(String),
    NewDefinition,
    SetEditorField {
        field: String,
        value: String,
    },
    ToggleComboPosition(usize),
    AddMacroStep,
    RemoveMacroStep(usize),
    MoveMacroStep {
        index: usize,
        delta: i32,
    },
    ApplyDefinition,
    RevertDefinition,
    DeleteDefinition,
    DownloadArtifacts(std::path::PathBuf),
    RevealDiagnostic {
        layer: LayerId,
        key: KeyPosition,
    },
    SelectLayer(LayerId),
    BeginLayerRename(LayerId),
    SetLayerName(String),
    ApplyLayerRename,
    CancelLayerRename,
    SelectKey(Option<KeyPosition>),
    NavigateKey {
        dx: f32,
        dy: f32,
    },
    SetBehavior(Behavior),
    SetBindingBehavior(String),
    SetBindingArgument {
        index: usize,
        value: String,
    },
    ChooseBindingArgument {
        index: usize,
        value: String,
    },
    ToggleBindingModifier {
        index: usize,
        modifier: String,
    },
    SetRawBinding(String),
    SetParameter {
        index: usize,
        value: String,
    },
    ApplyBinding,
    ChangeBinding {
        layer: LayerId,
        key: KeyPosition,
        binding: Binding,
    },
    Undo,
    Redo,
    ValidateDocument,
    SaveDocument,
    SaveAndBuild,
    SetAppearance(Appearance),
    SetReducedMotion(bool),
    SetLocalKeymapDirectory(Option<std::path::PathBuf>),
    StartBuild {
        fail: bool,
    },
    CancelBuild,
    BuildResult(BuildEvent),
    TogglePane(Pane),
    ResizePane {
        pane: Pane,
        size: f32,
    },
    ToggleFocusMode,
    ResetWorkspaceLayout,
    SelectBottomTab(BottomTab),
    WindowResized {
        width: f32,
        height: f32,
    },
    Navigate(Destination),
}

struct ProjectEdit {
    before: Box<Project>,
    after: Box<Project>,
}
#[derive(Debug, Clone)]
pub struct LayerNameDraft {
    pub layer: LayerId,
    pub name: String,
    pub error: Option<String>,
}

pub struct Application {
    pub(crate) nested_draft: Option<crate::editing::NestedDraft>,
    pub project: Project,
    pub original: Keymap,
    pub active_layer: LayerId,
    pub selected_key: Option<KeyPosition>,
    pub draft: Option<BindingDraft>,
    pub layer_name_draft: Option<LayerNameDraft>,
    pub diagnostics: Vec<Diagnostic>,
    pub workspace: Workspace,
    pub destination: Destination,
    pub build: Build,
    pub artifact_history: Vec<ArtifactSet>,
    pub build_environment: crate::build_environment::State,
    pub(crate) environment_generation: u64,
    pub(crate) resume_local_build: bool,
    pub status: String,
    undo: Vec<ProjectEdit>,
    redo: Vec<ProjectEdit>,
    pub(crate) saved_project: Project,
    pub connection: crate::github::Connection,
    pub editor: Option<crate::editor::Draft>,
    pub workflow_run: Option<crate::github::Run>,
    pub local_firmware: Option<std::path::PathBuf>,
    pub(crate) live_service: crate::github::Service,
    pub(crate) source_save_build: Option<crate::live::BuildTarget>,
    pub(crate) epoch: u64,
    pub(crate) next_build: u64,
    clock: Option<BuildClock>,
    pub(crate) storage: Option<StorageWorker>,
    pub(crate) session_cache_error: Option<String>,
}
impl Application {
    /// Headless, deterministic application instance with no filesystem side effects.
    pub fn sample() -> Result<Self> {
        Self::from_project(services::sample_project()?)
    }
    pub fn from_project(project: Project) -> Result<Self> {
        project.check_structure()?;
        let original = project.keymap.clone();
        let mut app = Self {
            nested_draft: None,
            saved_project: project.clone(),
            connection: crate::github::Connection::default(),
            editor: None,
            workflow_run: None,
            local_firmware: None,
            live_service: crate::github::Service::default(),
            source_save_build: None,
            project,
            original,
            active_layer: LayerId(0),
            selected_key: None,
            draft: None,
            layer_name_draft: None,
            diagnostics: vec![],
            workspace: Workspace::default(),
            destination: Destination::Keymap,
            build: Build::default(),
            artifact_history: Vec::new(),
            build_environment: Default::default(),
            environment_generation: 0,
            resume_local_build: false,
            status: "Prototype JSON · simulated services".into(),
            undo: vec![],
            redo: vec![],
            epoch: 0,
            next_build: 0,
            clock: None,
            storage: None,
            session_cache_error: None,
        };
        app.validate();
        Ok(app)
    }
    pub fn for_frontend(frontend: &str) -> Result<Self> {
        Self::with_storage(LocalStorage::for_frontend(frontend)?)
    }
    pub fn with_storage(storage: LocalStorage) -> Result<Self> {
        let mut app = match storage.load_project() {
            Ok(p) => Self::from_project(p)?,
            Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => Self::sample()?,
            Err(e) => return Err(e),
        };
        match storage.load_preferences() {
            Ok(p) => app.workspace.preferences = p,
            Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => app.status = e.to_string(),
        }
        app.storage = Some(StorageWorker::new(storage));
        Ok(app)
    }
    pub fn binding(&self, layer: LayerId, key: KeyPosition) -> Option<&Binding> {
        self.project.keymap.layers.get(layer.0)?.bindings.get(key.0)
    }
    pub fn selected_binding(&self) -> Option<&Binding> {
        self.binding(self.active_layer, self.selected_key?)
    }
    pub(crate) fn reset_draft(&mut self) {
        self.draft = self.selected_binding().map(BindingDraft::from);
    }
    pub fn binding_form(&self) -> Option<crate::bindings::Form> {
        self.draft.as_ref().map(|draft| {
            if let Some(reference) = &draft.reference {
                crate::bindings::Form {
                    reference: reference.clone(),
                    arguments: draft.parameters.clone(),
                }
            } else {
                let (reference, arguments) = crate::bindings::editing_parts(&draft.source());
                crate::bindings::Form {
                    reference,
                    arguments,
                }
            }
        })
    }
    fn set_binding_form(&mut self, form: crate::bindings::Form) {
        self.draft = Some(crate::bindings::draft(&form.reference, &form.arguments));
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn changes(&self) -> Vec<BindingChange> {
        self.project
            .keymap
            .layers
            .iter()
            .enumerate()
            .flat_map(|(l, layer)| {
                layer.bindings.iter().enumerate().filter_map(move |(k, b)| {
                    let before = self.original.layers.get(l)?.bindings.get(k)?;
                    (before != b).then(|| BindingChange {
                        layer: LayerId(l),
                        key: KeyPosition(k),
                        before: before.clone(),
                        after: b.clone(),
                    })
                })
            })
            .collect()
    }
    pub fn modified(&self, layer: LayerId, key: KeyPosition) -> bool {
        self.binding(layer, key)
            != self
                .original
                .layers
                .get(layer.0)
                .and_then(|l| l.bindings.get(key.0))
    }
    pub fn draft_diagnostics(&self) -> Vec<Diagnostic> {
        match (&self.draft, self.selected_key) {
            (Some(d), Some(key)) => match d.binding() {
                Ok(b) => {
                    let mut issues = self.binding_issues(&b, self.active_layer, key);
                    if let Some(form) = self.binding_form() {
                        issues.extend(
                            crate::bindings::validate_form(
                                &form,
                                &crate::bindings::catalog(self.project.zmk.as_ref()),
                                self.project.keymap.layers.len(),
                            )
                            .into_iter()
                            .map(|message| Diagnostic {
                                severity: Severity::Error,
                                message,
                                layer: self.active_layer,
                                key,
                            }),
                        );
                    }
                    issues
                }
                Err(e) => vec![Diagnostic {
                    severity: Severity::Error,
                    message: e.to_string(),
                    layer: self.active_layer,
                    key,
                }],
            },
            _ => vec![],
        }
    }
    pub(crate) fn validate(&mut self) {
        self.diagnostics.clear();
        for (layer_index, layer) in self.project.keymap.layers.iter().enumerate() {
            for (key_index, binding) in layer.bindings.iter().enumerate() {
                let issues =
                    self.binding_issues(binding, LayerId(layer_index), KeyPosition(key_index));
                self.diagnostics.extend(
                    issues.into_iter().filter(|_| {
                        self.project.zmk.is_none() || !matches!(binding, Binding::None)
                    }),
                );
            }
        }
    }
    fn binding_issues(
        &self,
        binding: &Binding,
        layer: LayerId,
        key: KeyPosition,
    ) -> Vec<Diagnostic> {
        let mut issues = validate_binding(binding, layer, key, self.project.keymap.layers.len());
        if self.project.zmk.is_some() {
            for issue in &mut issues {
                if issue.message.starts_with("Unknown key code:")
                    || issue.message.starts_with("Unknown tap key:")
                {
                    issue.severity = Severity::Warning;
                    issue
                        .message
                        .push_str(". This value must be defined by your firmware headers.");
                }
            }
        }
        issues
    }
    fn replace(&mut self, layer: LayerId, key: KeyPosition, binding: Binding) -> Result<()> {
        let mut source = self.project.zmk.clone();
        if let Some(doc) = &mut source {
            doc.replace_binding(layer.0, key.0, &binding.zmk())?;
        }
        let value = self
            .project
            .keymap
            .layers
            .get_mut(layer.0)
            .and_then(|layer| layer.bindings.get_mut(key.0))
            .ok_or(Error::InvalidSelection)?;
        *value = binding;
        self.project.zmk = source;
        self.validate();
        self.reset_draft();
        Ok(())
    }
    pub fn dispatch(&mut self, command: Command) -> Result<()> {
        let pending_binding = self.project.zmk.is_some()
            && self
                .draft
                .as_ref()
                .zip(self.selected_binding())
                .is_some_and(|(draft, binding)| draft.binding().as_ref().ok() != Some(binding));
        let pending_nested_binding = self.nested_draft_invalid();
        let changes_selection = match &command {
            Command::SelectKey(key) => *key != self.selected_key,
            Command::SelectLayer(layer) => *layer != self.active_layer,
            Command::NavigateKey { .. } | Command::RevealDiagnostic { .. } => true,
            Command::Navigate(destination) => {
                !matches!(destination, Destination::Settings | Destination::Repository)
                    && if pending_nested_binding {
                        self.editor.as_ref().is_none_or(|draft| {
                            crate::live::section(*destination) != Some(draft.section)
                        })
                    } else {
                        *destination != Destination::Keymap
                    }
            }
            Command::Undo | Command::Redo | Command::NewDefinition | Command::OpenDefinition(_) => {
                true
            }
            _ => false,
        };
        if (pending_binding || pending_nested_binding) && changes_selection {
            return Err(Error::InvalidDocument(
                "Finish or discard the invalid draft before changing selection".into(),
            ));
        }
        if matches!(command, Command::Undo | Command::Redo)
            && self.project.zmk.is_some()
            && self.editor.as_ref().is_some_and(|draft| draft.dirty())
        {
            return Err(Error::InvalidDocument(
                "Finish or discard the invalid definition draft before undo or redo".into(),
            ));
        }
        let choose_argument = matches!(command, Command::ChooseBindingArgument { .. });
        let preference_change = matches!(
            command,
            Command::TogglePane(_)
                | Command::StartBuild { .. }
                | Command::ResizePane { .. }
                | Command::ResetWorkspaceLayout
                | Command::SelectBottomTab(_)
                | Command::WindowResized { .. }
                | Command::SetAppearance(_)
                | Command::SetReducedMotion(_)
                | Command::SetLocalKeymapDirectory(_)
        );
        if self.dispatch_live(&command)? {
            return Ok(());
        }
        match command {
            Command::EditBinding { target, edit } => return self.edit_assignment(target, edit),
            Command::DiscardBindingDraft => {
                self.nested_draft = None;
                self.reset_draft();
            }
            Command::OpenDefinition(reference) => {
                let preview = self
                    .project
                    .zmk
                    .as_ref()
                    .and_then(|doc| crate::bindings::definition_preview(doc, &reference))
                    .ok_or(Error::InvalidSelection)?;
                let destination = if preview.section == crate::editor::Section::Macros {
                    Destination::Macros
                } else {
                    Destination::Behaviors
                };
                self.dispatch(Command::Navigate(destination))?;
                return self.dispatch(Command::SelectEditor(preview.path));
            }
            Command::ToggleEditorLayer { field, layer } => {
                if layer.0 >= self.project.keymap.layers.len() {
                    return Err(Error::InvalidSelection);
                }
                let editor = self.editor.as_mut().ok_or(Error::InvalidSelection)?;
                let mut layers = editor
                    .get(&field)
                    .split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                let value = layer.0.to_string();
                if layers.contains(&value) {
                    layers.retain(|l| l != &value);
                } else {
                    layers.push(value);
                }
                editor.set(&field, layers.join(" "))?;
                self.stage_definition();
            }
            Command::OpenSample => {
                self.epoch += 1;
                self.project = services::sample_project()?;
                self.original = self.project.keymap.clone();
                self.saved_project = self.project.clone();
                self.active_layer = LayerId(0);
                self.selected_key = None;
                self.undo.clear();
                self.redo.clear();
                self.draft = None;
                self.layer_name_draft = None;
                self.validate();
                self.cancel_build();
            }
            Command::RevealDiagnostic { layer, key } => {
                if self.binding(layer, key).is_none() {
                    return Err(Error::InvalidSelection);
                }
                self.active_layer = layer;
                self.selected_key = Some(key);
                self.destination = Destination::Keymap;
                self.reset_draft();
            }
            Command::SelectLayer(layer) => {
                if self
                    .editor
                    .as_ref()
                    .is_some_and(|d| d.section == crate::editor::Section::Encoders && d.dirty())
                {
                    return Err(Error::InvalidDocument(
                        "Complete or discard this encoder draft before changing layers".into(),
                    ));
                }
                if layer.0 >= self.project.keymap.layers.len() {
                    return Err(Error::InvalidSelection);
                }
                self.active_layer = layer;
                self.reset_draft();
                if self
                    .editor
                    .as_ref()
                    .is_some_and(|d| d.section == crate::editor::Section::Encoders)
                {
                    self.refresh_editor();
                }
            }
            Command::SelectKey(key) => {
                if key == self.selected_key && pending_binding {
                    return Ok(());
                }
                if key.is_some_and(|k| k.0 >= self.project.keyboard.keys.len()) {
                    return Err(Error::InvalidSelection);
                }
                self.selected_key = key;
                self.reset_draft();
            }
            Command::NavigateKey { dx, dy } => {
                self.selected_key = Some(
                    self.selected_key
                        .map(|k| self.project.keyboard.navigate(k, dx, dy))
                        .unwrap_or(KeyPosition(0)),
                );
                self.reset_draft();
            }
            Command::SetBehavior(behavior) => {
                self.draft
                    .as_mut()
                    .ok_or(Error::InvalidSelection)?
                    .change_behavior(behavior);
            }
            Command::SetBindingBehavior(reference) => {
                let catalog = crate::bindings::catalog(self.project.zmk.as_ref());
                let spec = catalog
                    .iter()
                    .find(|s| s.reference == reference)
                    .ok_or(Error::InvalidSelection)?;
                let mut arguments = spec
                    .parameters
                    .iter()
                    .map(|p| p.default.clone())
                    .collect::<Vec<_>>();
                if let Some(current) = self.binding_form()
                    && let Some(previous) =
                        catalog.iter().find(|s| s.reference == current.reference)
                    && let Some(index) = previous
                        .fields(&current.arguments)
                        .iter()
                        .rposition(|p| p.kind == crate::bindings::ParameterKind::KeyCode)
                    && let Some(value) = current.arguments.get(index)
                    && let Some(index) = spec
                        .parameters
                        .iter()
                        .rposition(|p| p.kind == crate::bindings::ParameterKind::KeyCode)
                {
                    arguments[index] = value.clone();
                }
                let arguments = arguments[..spec.fields(&arguments).len()].to_vec();
                self.set_binding_form(crate::bindings::Form {
                    reference,
                    arguments,
                });
            }
            Command::SetRawBinding(value) => {
                let draft = self.draft.as_mut().ok_or(Error::InvalidSelection)?;
                draft.behavior = Behavior::Custom;
                draft.reference = None;
                draft.parameters = vec![value, String::new()];
            }
            Command::SetBindingArgument { index, value }
            | Command::ChooseBindingArgument { index, value } => {
                let crate::bindings::Form {
                    reference,
                    mut arguments,
                } = self.binding_form().ok_or(Error::InvalidSelection)?;
                let catalog = crate::bindings::catalog(self.project.zmk.as_ref());
                let spec = catalog
                    .iter()
                    .find(|s| s.reference == reference)
                    .ok_or(Error::InvalidSelection)?;
                let fields = spec.fields(&arguments);
                let field = fields.get(index).ok_or(Error::InvalidSelection)?;
                arguments.resize(fields.len(), String::new());
                arguments[index] =
                    if choose_argument && field.kind == crate::bindings::ParameterKind::KeyCode {
                        crate::bindings::with_modifiers(
                            &value,
                            &crate::bindings::modifiers(&arguments[index]).1,
                        )
                    } else {
                        value
                    };
                let fields = spec.fields(&arguments);
                arguments.truncate(fields.len());
                while arguments.len() < fields.len() {
                    arguments.push(fields[arguments.len()].default.clone());
                }
                self.set_binding_form(crate::bindings::Form {
                    reference,
                    arguments,
                });
            }
            Command::ToggleBindingModifier { index, modifier } => {
                if !crate::bindings::MODIFIER_FUNCTIONS
                    .iter()
                    .any(|(function, _)| *function == modifier)
                {
                    return Err(Error::InvalidSelection);
                }
                let crate::bindings::Form {
                    reference,
                    arguments,
                } = self.binding_form().ok_or(Error::InvalidSelection)?;
                let spec = crate::bindings::catalog(self.project.zmk.as_ref())
                    .into_iter()
                    .find(|s| s.reference == reference)
                    .ok_or(Error::InvalidSelection)?;
                if !spec
                    .fields(&arguments)
                    .get(index)
                    .is_some_and(|p| p.kind == crate::bindings::ParameterKind::KeyCode)
                {
                    return Err(Error::InvalidSelection);
                }
                let (code, mut modifiers) = crate::bindings::modifiers(
                    arguments.get(index).map(String::as_str).unwrap_or("A"),
                );
                if modifiers.contains(&modifier) {
                    modifiers.retain(|m| m != &modifier);
                } else {
                    modifiers.push(modifier);
                }
                return self.dispatch(Command::SetBindingArgument {
                    index,
                    value: crate::bindings::with_modifiers(&code, &modifiers),
                });
            }
            Command::SetParameter { index, value } => {
                let field = self
                    .draft
                    .as_mut()
                    .and_then(|d| d.parameters.get_mut(index))
                    .ok_or(Error::InvalidSelection)?;
                *field = value;
            }
            Command::ApplyBinding => {
                let key = self.selected_key.ok_or(Error::InvalidSelection)?;
                let binding = self
                    .draft
                    .as_ref()
                    .ok_or(Error::InvalidSelection)?
                    .binding()?;
                if self
                    .draft_diagnostics()
                    .iter()
                    .any(|d| d.severity == Severity::Error)
                {
                    return Err(Error::InvalidDocument(
                        "Resolve inspector errors before applying".into(),
                    ));
                }
                return self.dispatch(Command::ChangeBinding {
                    layer: self.active_layer,
                    key,
                    binding,
                });
            }
            Command::ChangeBinding {
                layer,
                key,
                binding,
            } => {
                let before = self
                    .binding(layer, key)
                    .ok_or(Error::InvalidSelection)?
                    .clone();
                if before != binding {
                    let before_project = self.project.clone();
                    self.replace(layer, key, binding)?;
                    self.record_edit(before_project);
                }
            }
            Command::Undo => {
                if let Some(change) = self.undo.pop() {
                    self.restore_project((*change.before).clone());
                    self.redo.push(change);
                }
            }
            Command::Redo => {
                if let Some(change) = self.redo.pop() {
                    self.restore_project((*change.after).clone());
                    self.undo.push(change);
                }
            }
            Command::ValidateDocument => {
                self.validate();
                self.status = format!(
                    "{} diagnostics · prototype validation",
                    self.diagnostics.len()
                );
            }
            Command::SaveDocument | Command::SaveAndBuild => {
                if self.project.zmk.is_some() {
                    return self.save_source(matches!(command, Command::SaveAndBuild));
                }
                let build_after_save = matches!(command, Command::SaveAndBuild);
                self.storage
                    .as_mut()
                    .ok_or(Error::ServiceUnavailable)?
                    .submit(StorageRequest::Save {
                        epoch: self.epoch,
                        project: Box::new(self.project.clone()),
                        build_after_save,
                    })?;
                self.status = "Saving prototype JSON…".into();
            }
            Command::StartBuild { fail } => {
                if self.project.zmk.is_some() {
                    return self.save_source(true);
                }
                self.cancel_build();
                self.next_build += 1;
                self.build = Build {
                    id: self.next_build,
                    status: BuildStatus::Queued,
                    progress: 0,
                    output: vec!["SIMULATED build queued · no real firmware is compiled".into()],
                    artifacts: vec![],
                    problems: vec![],
                    current_target: None,
                };
                self.clock = Some(BuildClock::new(self.next_build, fail));
                self.workspace.preferences.bottom_tab = BottomTab::BuildOutput;
            }
            Command::CancelBuild => {
                if self.project.zmk.is_some() && self.build.status.active() {
                    self.live_service.cancel_build();
                    self.status = "Requesting build cancellation…".into();
                    self.build.output.push(self.status.clone());
                } else {
                    self.cancel_build();
                }
            }
            Command::BuildResult(event) => self.receive_build(event),
            Command::TogglePane(pane) => self.workspace.toggle(pane),
            Command::ResizePane { pane, size } => self.workspace.resize(pane, size),
            Command::ToggleFocusMode => self.workspace.focus_mode = !self.workspace.focus_mode,
            Command::ResetWorkspaceLayout => self.workspace.reset(),
            Command::SelectBottomTab(tab) => self.workspace.preferences.bottom_tab = tab,
            Command::WindowResized { width, height } => {
                if width.is_finite() && height.is_finite() {
                    self.workspace.preferences.window_width = width.max(900.);
                    self.workspace.preferences.window_height = height.max(600.);
                }
            }
            Command::Navigate(destination) => {
                if self.project.zmk.is_none() && crate::live::section(destination).is_some() {
                    return Err(Error::InvalidDocument(
                        "Open a keymap to edit this workspace".into(),
                    ));
                }
                if self.editor.as_ref().is_some_and(|draft| {
                    draft.dirty() && crate::live::section(destination) != Some(draft.section)
                }) && !matches!(destination, Destination::Settings | Destination::Repository)
                {
                    return Err(Error::InvalidDocument(
                        "Complete or discard the invalid definition draft before changing workspaces".into(),
                    ));
                }
                self.destination = destination;
                self.open_editor_for_destination();
            }
            Command::SetAppearance(appearance) => {
                self.workspace.preferences.appearance = appearance
            }
            Command::SetReducedMotion(reduced) => {
                self.workspace.preferences.reduced_motion = reduced
            }
            Command::SetLocalKeymapDirectory(directory) => {
                let directory = directory.map(|path| path.canonicalize()).transpose()?;
                if directory.as_ref().is_some_and(|path| !path.is_dir()) {
                    return Err(Error::InvalidDocument(
                        "Choose a folder for local keymaps".into(),
                    ));
                }
                self.workspace.preferences.local_keymap_directory = directory;
            }
            _ => return Err(Error::InvalidSelection),
        }
        if preference_change {
            self.persist_preferences()?;
        }
        Ok(())
    }
    pub fn execute(&mut self, command: Command) {
        let definition = matches!(command, Command::ApplyDefinition);
        let layer_rename = matches!(command, Command::ApplyLayerRename);
        let connection = matches!(
            command,
            Command::ConnectGitHub
                | Command::FindConfigurations
                | Command::BrowseRepository
                | Command::BrowseBranch(_)
                | Command::OpenRepository
                | Command::OpenLocal(_)
                | Command::SaveLocalCopy { .. }
                | Command::SaveAndBuildOnGitHub
        );
        if let Err(error) = self.dispatch(command) {
            self.status = error.to_string();
            if definition && let Some(draft) = &mut self.editor {
                draft.error = Some(self.status.clone());
            }
            if layer_rename && let Some(draft) = &mut self.layer_name_draft {
                draft.error = Some(self.status.clone());
            }
            if connection {
                self.connection.message = self.status.clone();
            }
        }
    }
    pub(crate) fn cancel_build(&mut self) {
        self.live_service.cancel_build();
        if self.build.status.active() {
            self.build.status = BuildStatus::Cancelled;
            self.build.output.push(
                if self.project.zmk.is_some() {
                    "Cancellation requested"
                } else {
                    "[simulated] Cancelled"
                }
                .into(),
            );
        }
        self.clock = None;
    }
    pub(crate) fn receive_build(&mut self, event: BuildEvent) {
        if event.operation != self.build.id || !self.build.status.active() {
            return;
        }
        // Only forward progress from a valid plan can transition the current operation.
        let allowed = matches!(
            (self.build.status, event.status),
            (
                BuildStatus::Queued,
                BuildStatus::Running | BuildStatus::Failed | BuildStatus::Cancelled
            ) | (
                BuildStatus::Running,
                BuildStatus::Running
                    | BuildStatus::Successful
                    | BuildStatus::Failed
                    | BuildStatus::Cancelled
            )
        );
        if !allowed || event.progress < self.build.progress {
            return;
        }
        self.build.status = event.status;
        self.build.progress = event.progress;
        if !event.line.is_empty() {
            self.record_build_output(event.line);
        }
        if event.status == BuildStatus::Failed
            && !self
                .build
                .problems
                .iter()
                .any(|p| p.severity == Severity::Error)
        {
            self.build.problems.push(BuildProblem {
                severity: Severity::Error,
                message: "Firmware build failed. See Output for details.".into(),
                target: self.build.current_target.clone(),
            });
        }
        self.build.artifacts = event.artifacts;
        if !self.build.status.active() {
            self.clock = None;
        }
        if self.build.status == BuildStatus::Successful && !self.build.artifacts.is_empty() {
            let set = ArtifactSet {
                build: self.build.id,
                configuration: self.project.name.clone(),
                directory: self.local_firmware.clone(),
                artifacts: self.build.artifacts.clone(),
            };
            self.artifact_history
                .retain(|previous| previous.build != set.build);
            self.artifact_history.push(set);
            if self.artifact_history.len() > 5 {
                self.artifact_history.remove(0);
            }
        }
    }
    pub fn persist_preferences(&mut self) -> Result<()> {
        if let Some(worker) = &mut self.storage {
            worker.submit(StorageRequest::Preferences(
                self.workspace.preferences.clone(),
            ))?;
        }
        Ok(())
    }
    pub(crate) fn record_build_output(&mut self, output: String) {
        for line in output.lines() {
            if let Some(target) = line
                .strip_prefix("Building target ")
                .and_then(|line| line.split_once(": ").map(|(_, target)| target))
            {
                self.build.current_target = Some(target.trim().to_string());
            }
            let message = line.trim();
            if message.starts_with("Traceback (") {
                continue;
            }
            if let Some(severity) = build_line_severity(line)
                && !self.build.problems.iter().any(|p| p.message == message)
                && !(message.contains("Local firmware build failed")
                    && self
                        .build
                        .problems
                        .iter()
                        .any(|p| p.severity == Severity::Error))
            {
                self.build.problems.push(BuildProblem {
                    severity,
                    message: message.into(),
                    target: self.build.current_target.clone(),
                });
            }
        }
        // Store individual rows so frontends can index visible lines directly,
        // without flattening the entire compiler transcript on every frame.
        self.build.output.extend(output.lines().map(str::to_owned));
    }
    pub fn problem_count(&self) -> usize {
        self.diagnostics.len() + self.build.problems.len()
    }
    pub fn needs_poll(&self) -> bool {
        self.build_environment.busy()
            || self.connection.busy
            || self.build.status.active()
            || self.storage.as_ref().is_some_and(|w| w.pending > 0)
    }
    /// A close barrier for the existing storage worker, not a second save path.
    pub fn finish_pending_storage(&self) -> Result<Option<std::sync::mpsc::Receiver<()>>> {
        self.storage
            .as_ref()
            .filter(|worker| worker.pending > 0)
            .map(StorageWorker::flush)
            .transpose()
    }
    pub fn session_cache_error(&self) -> Option<&str> {
        self.session_cache_error.as_deref()
    }
    /// Nonblocking service completion delivery; call from the frontend event loop.
    pub fn poll(&mut self) -> bool {
        let mut changed = self.poll_live();
        if let Some(event) = self.clock.as_mut().and_then(BuildClock::poll) {
            self.receive_build(event);
            changed = true;
        }
        while let Some(event) = self.storage.as_mut().and_then(StorageWorker::poll) {
            changed = true;
            match event {
                StorageEvent::Saved {
                    epoch,
                    keymap,
                    build_after_save,
                    project,
                } if epoch == self.epoch => {
                    self.original = keymap;
                    self.saved_project = *project;
                    self.status = "Saved prototype JSON locally".into();
                    if build_after_save {
                        self.execute(Command::StartBuild { fail: false });
                    }
                }
                StorageEvent::Saved { .. } | StorageEvent::PreferencesSaved => {}
                StorageEvent::CacheSaved => self.session_cache_error = None,
                StorageEvent::CacheFailed(error) => {
                    self.status = format!("Could not remember this keymap: {error}");
                    self.session_cache_error = Some(error);
                }
                StorageEvent::Failed(error) => self.status = error,
            }
        }
        changed
    }
}

impl Application {
    pub fn has_changes(&self) -> bool {
        self.project.keymap != self.original
            || self.project.zmk.as_ref().map(|d| &d.source)
                != self.saved_project.zmk.as_ref().map(|d| &d.source)
    }
    pub fn pending_count(&self) -> usize {
        let keys = self.changes().len();
        keys.max(usize::from(self.has_changes()))
    }
    pub(crate) fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
    pub(crate) fn record_edit(&mut self, before: Project) {
        if before != self.project {
            self.undo.push(ProjectEdit {
                before: Box::new(before),
                after: Box::new(self.project.clone()),
            });
            self.redo.clear();
        }
    }
    pub(crate) fn restore_project(&mut self, mut project: Project) {
        self.layer_name_draft = None;
        self.nested_draft = None;
        if let (Some(current), Some(restored)) = (&self.project.zmk, &mut project.zmk) {
            restored.origin = current.origin.clone();
        }
        self.project = project;
        self.active_layer.0 = self
            .active_layer
            .0
            .min(self.project.keymap.layers.len() - 1);
        self.validate();
        self.reset_draft();
        self.refresh_editor();
    }
}
