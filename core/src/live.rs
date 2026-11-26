use crate::{
    Application, Build, BuildStatus, Command, Destination, Error, LayerId, Result,
    editor::{self, Section},
    github::{self, Reply, Request},
    services::StorageRequest,
    zmk::Origin,
};

pub(crate) enum BuildTarget {
    Local,
    GitHub,
}

pub fn section(destination: Destination) -> Option<Section> {
    match destination {
        Destination::Combos => Some(Section::Combos),
        Destination::Macros => Some(Section::Macros),
        Destination::Behaviors => Some(Section::Behaviors),
        Destination::ConditionalLayers => Some(Section::ConditionalLayers),
        Destination::Encoders => Some(Section::Encoders),
        _ => None,
    }
}
impl Application {
    pub fn initialize_live_ui(&mut self) {
        if let Some(doc) = &self.project.zmk {
            self.build.output = vec![match &doc.origin {
                Origin::GitHub {
                    repository, branch, ..
                } => format!(
                    "Connected source: {repository} · {branch}. Save a local configuration copy to build on this machine. GitHub save/build remains available under Open configuration."
                ),
                Origin::Local { path, .. } => format!(
                    "Local keymap: {}. Save and build writes this file and builds locally with Docker or Podman. Save file writes without building.",
                    path.display()
                ),
            }];
            if let Origin::GitHub {
                repository,
                branch,
                path,
                workflow,
                ..
            } = &doc.origin
            {
                self.connection.repository = repository.clone();
                self.connection.branch = branch.clone();
                self.connection.file = path.clone();
                self.connection.workflow = workflow.map(|w| w.to_string()).unwrap_or_default();
            }
            self.status = "Keymap loaded".into();
        } else {
            self.destination = Destination::Repository;
            self.diagnostics.clear();
            self.build.output =
                vec!["Open a GitHub configuration or a local keymap to begin.".into()];
            self.status = "No configuration open".into();
        }
    }
    fn begin_repository(&mut self, request: Request, message: &str) -> Result<()> {
        if self.connection.busy {
            return Err(Error::InvalidDocument(
                "Wait for the current repository operation to finish".into(),
            ));
        }
        self.connection.request += 1;
        self.connection.busy = true;
        self.connection.message = message.into();
        self.status = message.into();
        self.live_service.submit(self.connection.request, request);
        Ok(())
    }
    fn can_open_source(&self) -> Result<()> {
        if self.has_changes() || self.has_unapplied_draft() {
            return Err(Error::InvalidDocument("Apply and save your edits, or explicitly discard them, before opening another keymap.".into()));
        }
        if self.build.status.active() {
            return Err(Error::InvalidDocument(
                "Cancel the running build before opening another keymap.".into(),
            ));
        }
        Ok(())
    }
    pub fn has_unapplied_draft(&self) -> bool {
        self.layer_name_draft.as_ref().is_some_and(|d| {
            self.project
                .keymap
                .layers
                .get(d.layer.0)
                .is_some_and(|layer| d.name.trim() != layer.name)
        }) || self.editor.as_ref().is_some_and(|d| d.dirty())
            || self
                .draft
                .as_ref()
                .zip(self.selected_binding())
                .is_some_and(|(d, b)| d.binding().as_ref().ok() != Some(b))
    }
    pub(crate) fn dispatch_live(&mut self, command: &Command) -> Result<bool> {
        match command {
            Command::CheckBuildEnvironment => {
                if !self.build_environment.busy() {
                    self.environment_generation += 1;
                    self.build_environment = crate::build_environment::State::Checking;
                    self.live_service.submit(
                        self.connection.request,
                        Request::BuildEnvironment {
                            generation: self.environment_generation,
                            prepare: false,
                        },
                    );
                }
            }
            Command::PrepareBuildEnvironment => {
                if let crate::build_environment::State::NeedsImage { engine } =
                    &self.build_environment
                {
                    self.environment_generation += 1;
                    self.build_environment = crate::build_environment::State::Installing {
                        engine: engine.clone(),
                    };
                    self.workspace.preferences.bottom.visible = true;
                    self.workspace.preferences.bottom_tab = crate::BottomTab::BuildOutput;
                    self.live_service.submit(
                        self.connection.request,
                        Request::BuildEnvironment {
                            generation: self.environment_generation,
                            prepare: true,
                        },
                    );
                }
            }
            Command::DismissBuildSetup => self.resume_local_build = false,
            Command::BeginLayerRename(layer) => {
                if self
                    .layer_name_draft
                    .as_ref()
                    .is_some_and(|d| d.layer == *layer)
                {
                    return Ok(true);
                }
                if self.project.zmk.is_none() {
                    return Err(Error::InvalidDocument(
                        "Open a configuration before renaming a layer".into(),
                    ));
                }
                if self.layer_name_draft.as_ref().is_some_and(|d| {
                    d.layer != *layer
                        && self
                            .project
                            .keymap
                            .layers
                            .get(d.layer.0)
                            .is_some_and(|l| d.name.trim() != l.name)
                }) {
                    return Err(Error::InvalidDocument(
                        "Finish or cancel the current layer name first".into(),
                    ));
                }
                let name = self
                    .project
                    .keymap
                    .layers
                    .get(layer.0)
                    .ok_or(Error::InvalidSelection)?
                    .name
                    .clone();
                self.layer_name_draft = Some(crate::application::LayerNameDraft {
                    layer: *layer,
                    name,
                    error: None,
                });
            }
            Command::SetLayerName(name) => {
                let draft = self
                    .layer_name_draft
                    .as_mut()
                    .ok_or(Error::InvalidSelection)?;
                draft.name = name.clone();
                draft.error = None;
            }
            Command::CancelLayerRename => self.layer_name_draft = None,
            Command::ApplyLayerRename => {
                let draft = self
                    .layer_name_draft
                    .as_ref()
                    .ok_or(Error::InvalidSelection)?;
                let name = draft.name.trim();
                if name.is_empty() || name.chars().any(char::is_control) {
                    return Err(Error::InvalidDocument(
                        "Enter a nonempty layer name without control characters".into(),
                    ));
                }
                let current = self
                    .project
                    .keymap
                    .layers
                    .get(draft.layer.0)
                    .ok_or(Error::InvalidSelection)?;
                if current.name == name {
                    self.layer_name_draft = None;
                    return Ok(true);
                }
                if self
                    .project
                    .keymap
                    .layers
                    .iter()
                    .enumerate()
                    .any(|(i, l)| i != draft.layer.0 && l.name.eq_ignore_ascii_case(name))
                {
                    return Err(Error::InvalidDocument(
                        "Another layer already uses that name".into(),
                    ));
                }
                let mut doc = self.project.zmk.clone().ok_or(Error::InvalidSelection)?;
                doc.rename_layer(draft.layer.0, name)?;
                let before = self.project.clone();
                self.project.keymap.layers = doc.layers()?;
                self.project.zmk = Some(doc);
                self.record_edit(before);
                self.layer_name_draft = None;
                self.validate();
                self.status = "Layer renamed. Save to update the configuration.".into();
            }
            Command::ConnectGitHub => {
                self.begin_repository(Request::Account, "Checking your GitHub CLI account…")?
            }
            Command::FindConfigurations => {
                let repository = github::repository(&self.connection.repository)?;
                self.begin_repository(
                    Request::FindConfigurations { repository },
                    "Looking for ZMK configuration repositories in this account…",
                )?;
            }
            Command::SetConnection { field, value } => match field.as_str() {
                "repository" => {
                    if self.connection.repository != *value {
                        self.connection.repository = value.clone();
                        self.connection.branches.clear();
                        self.connection.branch.clear();
                        self.connection.files.clear();
                        self.connection.file.clear();
                        self.connection.workflows.clear();
                        self.connection.workflow.clear();
                        self.connection.configurations.clear();
                        self.connection.browsed = false;
                        self.connection.editor_repository = false;
                        self.connection.message.clear();
                    }
                }
                "file" => self.connection.file = value.clone(),
                "workflow" => self.connection.workflow = value.clone(),
                _ => return Err(Error::InvalidSelection),
            },
            Command::BrowseRepository => {
                let repo = github::repository(&self.connection.repository)?;
                self.begin_repository(
                    Request::Browse {
                        repository: repo,
                        branch: String::new(),
                    },
                    "Finding branches and keymaps…",
                )?;
            }
            Command::BrowseBranch(branch) => {
                let repo = github::repository(&self.connection.repository)?;
                self.begin_repository(
                    Request::Browse {
                        repository: repo,
                        branch: branch.clone(),
                    },
                    "Reading this branch…",
                )?;
            }
            Command::OpenRepository => {
                self.can_open_source()?;
                if self.connection.file.is_empty() || self.connection.branch.is_empty() {
                    return Err(Error::InvalidDocument(
                        "Choose a repository branch containing a .keymap file".into(),
                    ));
                }
                let workflow = self
                    .connection
                    .workflows
                    .iter()
                    .find(|w| w.path == self.connection.workflow)
                    .map(|w| w.id);
                self.begin_repository(
                    Request::Open {
                        repository: self.connection.repository.clone(),
                        branch: self.connection.branch.clone(),
                        path: self.connection.file.clone(),
                        workflow,
                    },
                    "Loading ZMK keymap…",
                )?;
            }
            Command::ReloadSource => {
                self.can_open_source()?;
                let origin = self
                    .project
                    .zmk
                    .as_ref()
                    .ok_or(Error::InvalidSelection)?
                    .origin
                    .clone();
                let request = match origin {
                    Origin::Local { path, .. } => Request::Local(path),
                    Origin::GitHub {
                        repository,
                        branch,
                        path,
                        workflow,
                        ..
                    } => Request::Open {
                        repository,
                        branch,
                        path,
                        workflow,
                    },
                };
                self.begin_repository(request, "Refreshing keymap from its source…")?;
            }
            Command::OpenLocal(path) => {
                self.can_open_source()?;
                self.begin_repository(Request::Local(path.clone()), "Opening local ZMK keymap…")?;
            }
            Command::SaveLocalCopy { directory, build } => {
                if self.has_unapplied_draft() {
                    return Err(Error::InvalidDocument(
                        "Apply or revert the current draft before saving a local copy".into(),
                    ));
                }
                if self.build.status.active() {
                    return Err(Error::InvalidDocument(
                        "Wait for the running build before changing the source location".into(),
                    ));
                }
                self.begin_repository(
                    Request::LocalCopy {
                        project: Box::new(self.project.clone()),
                        directory: directory.clone(),
                    },
                    "Saving a local configuration copy…",
                )?;
                self.source_save_build = build.then_some(BuildTarget::Local);
            }
            Command::SaveAndBuildOnGitHub => {
                if !self
                    .project
                    .zmk
                    .as_ref()
                    .is_some_and(|d| matches!(d.origin, Origin::GitHub { .. }))
                {
                    return Err(Error::InvalidDocument(
                        "Open a GitHub configuration to save and build on GitHub".into(),
                    ));
                }
                self.save_source_with_target(Some(BuildTarget::GitHub))?;
            }
            Command::DiscardChanges => {
                self.resume_local_build = false;
                let saved = self.saved_project.clone();
                self.restore_project(saved);
                self.editor = None;
                self.layer_name_draft = None;
                self.clear_history();
                self.reset_draft();
                self.status = "Uncommitted edits discarded".into();
            }
            Command::SelectEditor(id) => {
                if let Some(section) = section(self.destination) {
                    if self.editor.as_ref().is_some_and(|d| d.dirty()) {
                        return Err(Error::InvalidDocument(
                            "Apply or revert the current definition draft first".into(),
                        ));
                    }
                    self.editor = Some(editor::draft(
                        &self.project,
                        section,
                        Some(id),
                        self.active_layer.0,
                    )?);
                }
            }
            Command::NewDefinition => {
                let section = section(self.destination).ok_or(Error::InvalidSelection)?;
                if section == Section::Encoders {
                    return Err(Error::InvalidDocument(
                        "Sensor bindings are discovered from the loaded keymap".into(),
                    ));
                }
                if self.editor.as_ref().is_some_and(|d| d.dirty()) {
                    return Err(Error::InvalidDocument(
                        "Apply or revert the current definition draft first".into(),
                    ));
                }
                self.editor = Some(editor::draft(
                    &self.project,
                    section,
                    None,
                    self.active_layer.0,
                )?);
            }
            Command::SetEditorField { field, value } => {
                if field == "behavior-type"
                    && self
                        .editor
                        .as_ref()
                        .is_some_and(|d| d.section == Section::Behaviors && d.node.is_none())
                {
                    let name = self.editor.as_ref().unwrap().get("name").to_string();
                    let mut draft =
                        editor::new_behavior(&self.project, value, self.active_layer.0)?;
                    draft.set("name", name)?;
                    self.editor = Some(draft);
                } else {
                    self.editor
                        .as_mut()
                        .ok_or(Error::InvalidSelection)?
                        .set(field, value.clone())?;
                }
            }
            Command::ToggleComboPosition(position) => {
                self.editor
                    .as_mut()
                    .ok_or(Error::InvalidSelection)?
                    .toggle_position(*position);
            }
            Command::AddMacroStep => self
                .editor
                .as_mut()
                .ok_or(Error::InvalidSelection)?
                .add_step(),
            Command::RemoveMacroStep(index) => self
                .editor
                .as_mut()
                .ok_or(Error::InvalidSelection)?
                .remove_step(*index),
            Command::MoveMacroStep { index, delta } => self
                .editor
                .as_mut()
                .ok_or(Error::InvalidSelection)?
                .move_step(*index, *delta)?,
            Command::ApplyDefinition => {
                let draft = self.editor.as_ref().ok_or(Error::InvalidSelection)?.clone();
                if !draft.dirty() {
                    return Ok(true);
                }
                let doc = editor::apply(&self.project, &draft)?;
                let before = self.project.clone();
                self.project.keymap.layers = doc.layers()?;
                self.project.zmk = Some(doc);
                self.record_edit(before);
                self.validate();
                let selected = editor::entries(&self.project, draft.section, self.active_layer.0)
                    .ok()
                    .and_then(|entries| {
                        entries
                            .into_iter()
                            .find(|e| e.name == draft.get("name").trim())
                            .map(|e| e.id)
                    })
                    .or(draft.node.clone());
                self.editor = selected.and_then(|id| {
                    editor::draft(&self.project, draft.section, Some(&id), self.active_layer.0).ok()
                });
                self.status = "Definition staged. Save to update the configuration.".into();
            }
            Command::RevertDefinition => {
                if self.editor.as_ref().is_some_and(|d| d.node.is_none()) {
                    self.editor = None;
                    self.open_editor_for_destination();
                } else {
                    self.refresh_editor();
                }
            }
            Command::DeleteDefinition => {
                let draft = self.editor.as_ref().ok_or(Error::InvalidSelection)?;
                let doc = editor::delete(&self.project, draft)?;
                let before = self.project.clone();
                self.project.zmk = Some(doc);
                self.record_edit(before);
                self.editor = None;
                self.open_editor_for_destination();
                self.status = "Definition deleted. Undo restores it.".into();
            }
            Command::DownloadArtifacts(directory) => {
                let run = self.workflow_run.clone().ok_or_else(|| {
                    Error::InvalidDocument("No GitHub workflow artifacts are available".into())
                })?;
                self.begin_repository(
                    Request::Download {
                        run,
                        directory: directory.clone(),
                    },
                    "Downloading firmware artifacts…",
                )?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
    pub(crate) fn refresh_editor(&mut self) {
        if let Some(draft) = &self.editor {
            self.editor = editor::draft(
                &self.project,
                draft.section,
                draft.node.as_deref(),
                self.active_layer.0,
            )
            .ok();
        }
    }
    pub(crate) fn open_editor_for_destination(&mut self) {
        let Some(section) = section(self.destination) else {
            return;
        };
        if self.editor.as_ref().is_some_and(|d| d.section == section) {
            return;
        }
        self.editor = editor::entries(&self.project, section, self.active_layer.0)
            .ok()
            .and_then(|entries| {
                entries.first().and_then(|e| {
                    editor::draft(&self.project, section, Some(&e.id), self.active_layer.0).ok()
                })
            });
    }
    pub(crate) fn save_source(&mut self, build: bool) -> Result<()> {
        self.save_source_with_target(build.then_some(BuildTarget::Local))
    }
    fn save_source_with_target(&mut self, target: Option<BuildTarget>) -> Result<()> {
        if self.has_unapplied_draft() {
            return Err(Error::InvalidDocument(
                "Apply your binding or definition draft before saving".into(),
            ));
        }
        let github = self
            .project
            .zmk
            .as_ref()
            .is_some_and(|doc| matches!(doc.origin, Origin::GitHub { .. }));
        if matches!(target, Some(BuildTarget::Local)) && github {
            return Err(Error::InvalidDocument(
                "Choose Save local copy first. Local builds never commit your edits to GitHub."
                    .into(),
            ));
        }
        if self.build.status.active() && target.is_some() {
            return Err(Error::InvalidDocument("A build is already running".into()));
        }
        if !self.has_changes() {
            return match target {
                Some(BuildTarget::Local) => self.start_local_build(),
                Some(BuildTarget::GitHub) => self.start_live_build(true),
                None => {
                    self.status = "No changes to save".into();
                    Ok(())
                }
            };
        }
        self.begin_repository(
            Request::Save {
                project: Box::new(self.project.clone()),
            },
            if github {
                "Saving keymap to GitHub…"
            } else {
                "Saving local keymap…"
            },
        )?;
        self.source_save_build = target;
        Ok(())
    }
    pub(crate) fn start_local_build(&mut self) -> Result<()> {
        if self.connection.busy || self.build.status.active() {
            return Err(Error::InvalidDocument(
                "Wait for the current operation to finish".into(),
            ));
        }
        if !self.build_environment.ready() {
            self.resume_local_build = true;
            self.dispatch_live(&Command::CheckBuildEnvironment)?;
            return Ok(());
        }
        let Some(Origin::Local { path, saved_source }) =
            self.project.zmk.as_ref().map(|d| &d.origin)
        else {
            return Err(Error::InvalidDocument(
                "Save a local configuration copy before building locally".into(),
            ));
        };
        let path = path.clone();
        let source = saved_source.clone();
        self.next_build += 1;
        self.build = Build {
            id: self.next_build,
            status: BuildStatus::Queued,
            progress: 0,
            output: vec![],
            artifacts: vec![],
            problems: vec![],
        };
        self.workflow_run = None;
        self.local_firmware = None;
        self.workspace.preferences.bottom.visible = true;
        self.workspace.preferences.bottom_tab = crate::BottomTab::BuildOutput;
        self.persist_preferences()?;
        self.live_service.submit(
            self.connection.request,
            Request::LocalBuild {
                path,
                source,
                operation: self.next_build,
            },
        );
        Ok(())
    }
    pub(crate) fn start_live_build(&mut self, dispatch: bool) -> Result<()> {
        if self.connection.busy {
            return Err(Error::InvalidDocument(
                "Wait for the repository operation to finish".into(),
            ));
        }
        let origin = self
            .project
            .zmk
            .as_ref()
            .ok_or(Error::InvalidSelection)?
            .origin
            .clone();
        if !matches!(origin, Origin::GitHub { .. }) {
            return Err(Error::InvalidDocument("Local keymaps save to disk. GitHub firmware builds require a connected repository.".into()));
        }
        self.cancel_build();
        self.next_build += 1;
        self.build = Build {
            id: self.next_build,
            status: BuildStatus::Queued,
            progress: 0,
            output: vec![
                if dispatch {
                    "Starting the selected GitHub workflow…"
                } else {
                    "Keymap committed. Waiting for its firmware workflow…"
                }
                .into(),
            ],
            artifacts: vec![],
            problems: vec![],
        };
        self.workflow_run = None;
        self.local_firmware = None;
        self.workspace.preferences.bottom.visible = true;
        self.workspace.preferences.bottom_tab = crate::BottomTab::BuildOutput;
        self.persist_preferences()?;
        self.live_service.submit(
            self.connection.request,
            Request::Build {
                origin,
                dispatch,
                operation: self.next_build,
            },
        );
        Ok(())
    }
    fn cache_source(&mut self) {
        if let Some(worker) = &mut self.storage
            && let Err(error) = worker.submit(StorageRequest::CacheProject(Box::new(
                self.saved_project.clone(),
            )))
        {
            self.status = format!("Keymap saved, but the session cache failed: {error}");
            self.session_cache_error = Some(error.to_string());
        }
    }
    pub(crate) fn poll_live(&mut self) -> bool {
        let mut changed = false;
        // Yield to the frontend between bursts of compiler/setup output.
        for _ in 0..16 {
            let Some(event) = self.live_service.poll() else {
                break;
            };
            changed = true;
            if let Reply::BuildEnvironment {
                generation,
                state,
                line,
            } = event.reply
            {
                if generation == self.environment_generation {
                    self.build_environment = state;
                    if !line.is_empty() {
                        self.record_build_output(line);
                    }
                    if self.build_environment.ready() && self.resume_local_build {
                        self.resume_local_build = false;
                        if let Err(error) = self.start_local_build() {
                            self.status = error.to_string();
                        }
                    }
                }
                continue;
            }
            if let Reply::LocalBuild { event, directory } = event.reply {
                if event.operation == self.build.id && self.build.status.active() {
                    if directory.is_some() {
                        self.local_firmware = directory;
                    }
                    self.receive_build(event);
                    if self.build.status == BuildStatus::Failed {
                        let _ = self.dispatch_live(&Command::CheckBuildEnvironment);
                    }
                }
                continue;
            }
            if let Reply::Build { event, run } = event.reply {
                if event.operation == self.build.id && self.build.status.active() {
                    if run.is_some() {
                        self.workflow_run = run;
                    }
                    self.receive_build(event);
                }
                continue;
            }
            if event.request != self.connection.request {
                continue;
            }
            self.connection.busy = false;
            let result = (|| -> Result<()> {
                let copied = matches!(event.reply, Reply::LocalCopy(_));
                match event.reply {
                    Reply::Account {
                        login,
                        repositories,
                    } => {
                        self.connection.message =
                            format!("Signed in as {login}. Choose a configuration repository.");
                        self.connection.account = Some(login);
                        self.connection.repositories = repositories;
                        self.status = self.connection.message.clone();
                    }
                    Reply::Browser {
                        repository,
                        branches,
                        branch,
                        files,
                        workflows,
                        editor_repository,
                    } => {
                        self.connection.repository = repository;
                        self.connection.branches = branches;
                        self.connection.branch = branch;
                        self.connection.file = files.first().cloned().unwrap_or_default();
                        self.connection.browsed = true;
                        self.connection.editor_repository = editor_repository;
                        self.connection.configurations.clear();
                        self.connection.message = if files.is_empty() {
                            if editor_repository {
                                "This repository contains the website's editor software. Your personal keymap is stored in a separate ZMK configuration repository. Find configurations below, or choose another repository.".into()
                            } else {
                                "No .keymap files on this branch. Choose another branch or configuration repository.".into()
                            }
                        } else {
                            format!("Found {} keymap file(s)", files.len())
                        };
                        self.connection.files = files;
                        self.connection.workflow = workflows
                            .iter()
                            .find(|w| w.path.to_lowercase().contains("build"))
                            .or(workflows.first())
                            .map(|w| w.path.clone())
                            .unwrap_or_default();
                        self.connection.workflows = workflows;
                        self.status = self.connection.message.clone();
                    }
                    Reply::Configurations {
                        configurations,
                        inspected,
                        candidates,
                        unavailable,
                    } => {
                        self.connection.message = format!(
                            "Found {} configuration repositories with .keymap files. Checked {inspected} of {candidates} repositories with ZMK/keyboard-related names on their default branches. You can also enter any repository directly.",
                            configurations.len()
                        );
                        if unavailable > 0 {
                            self.connection.message.push_str(&format!(
                                " {unavailable} repositories could not be inspected."
                            ));
                        }
                        self.connection.configurations = configurations;
                        self.status = self.connection.message.clone();
                    }
                    Reply::Open(document) => {
                        self.resume_local_build = false;
                        let project = document.project(self.project.keyboard.clone())?;
                        self.cancel_build();
                        self.epoch += 1;
                        self.project = project;
                        self.original = self.project.keymap.clone();
                        self.saved_project = self.project.clone();
                        self.clear_history();
                        self.active_layer = LayerId(0);
                        self.selected_key = None;
                        self.draft = None;
                        self.layer_name_draft = None;
                        self.editor = None;
                        self.destination = Destination::Keymap;
                        self.build = Build::default();
                        self.workflow_run = None;
                        self.local_firmware = None;
                        self.validate();
                        self.initialize_live_ui();
                        self.cache_source();
                        self.connection.message = "Keymap loaded".into();
                    }
                    Reply::Saved(project) | Reply::LocalCopy(project) => {
                        if let (Some(current), Some(saved)) = (&mut self.project.zmk, &project.zmk)
                        {
                            current.origin = saved.origin.clone();
                        }
                        self.original = project.keymap.clone();
                        self.saved_project = *project;
                        if copied {
                            self.resume_local_build = false;
                            self.initialize_live_ui();
                            self.workflow_run = None;
                            self.local_firmware = None;
                        }
                        self.status = match self.project.zmk.as_ref().map(|d| &d.origin) {
                            Some(Origin::Local { path, .. }) => {
                                format!("Keymap saved locally: {}", path.display())
                            }
                            _ => "Keymap saved to GitHub".into(),
                        };
                        self.connection.message = self.status.clone();
                        self.cache_source();
                        if let Some(target) = self.source_save_build.take() {
                            match target {
                                BuildTarget::Local => self.start_local_build()?,
                                BuildTarget::GitHub => self.start_live_build(false)?,
                            }
                        }
                    }
                    Reply::Downloaded(path) => {
                        self.status =
                            format!("Firmware artifacts downloaded to {}", path.display());
                        self.connection.message = self.status.clone();
                    }
                    Reply::Failed(message) => {
                        self.source_save_build = None;
                        self.status = message.clone();
                        self.connection.message = message;
                    }
                    Reply::Build { .. }
                    | Reply::LocalBuild { .. }
                    | Reply::BuildEnvironment { .. } => unreachable!(),
                }
                Ok(())
            })();
            if let Err(error) = result {
                self.status = error.to_string();
                self.connection.message = self.status.clone();
            }
        }
        changed
    }
}
