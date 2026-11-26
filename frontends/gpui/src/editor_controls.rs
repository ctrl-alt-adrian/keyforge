//! GPUI control entities mirror the core's drafts; they never own documents.
use crate::app::Keyforge;
use gpui::*;
use gpui_component::{
    input::{InputEvent, InputState},
    select::{SearchableVec, SelectEvent, SelectState},
};
use keyforge_core::{Command, editor::Kind};

#[derive(Clone, Copy)]
pub(crate) enum PathAction {
    Open,
    OpenFolder,
    Download,
    SaveLocal { build: bool },
}

impl Keyforge {
    pub(crate) fn install_live_controls(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._subscriptions.push(cx.subscribe_in(
            &self.layer_name_input,
            window,
            |this: &mut Self, input, event, window, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = input.read(cx).value().to_string();
                    if this
                        .core
                        .layer_name_draft
                        .as_ref()
                        .is_some_and(|d| d.name != value)
                    {
                        this.command(Command::SetLayerName(value), window, cx);
                    }
                }
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.command(Command::ApplyLayerRename, window, cx);
                    if this.core.layer_name_draft.is_none() {
                        this.canvas_focus.focus(window);
                    }
                }
            },
        ));
        self._subscriptions.push(cx.subscribe_in(
            &self.repository_input,
            window,
            |this: &mut Self, input, event, window, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = input.read(cx).value().to_string();
                    if this.core.connection.repository != value {
                        this.command(
                            Command::SetConnection {
                                field: "repository".into(),
                                value,
                            },
                            window,
                            cx,
                        );
                    }
                }
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.command(Command::BrowseRepository, window, cx);
                }
            },
        ));
        for (index, state) in self.repository_choices.iter().enumerate() {
            self._subscriptions.push(cx.subscribe_in(
                state,
                window,
                move |this: &mut Self, _, event, window, cx| {
                    if let SelectEvent::Confirm(Some(value)) = event {
                        let command = match index {
                            0 if this.core.connection.repository != *value => {
                                Some(Command::SetConnection {
                                    field: "repository".into(),
                                    value: value.clone(),
                                })
                            }
                            1 if this.core.connection.branch != *value => {
                                Some(Command::BrowseBranch(value.clone()))
                            }
                            2 if this.core.connection.file != *value => {
                                Some(Command::SetConnection {
                                    field: "file".into(),
                                    value: value.clone(),
                                })
                            }
                            3 if this.core.connection.workflow != *value => {
                                Some(Command::SetConnection {
                                    field: "workflow".into(),
                                    value: value.clone(),
                                })
                            }
                            _ => None,
                        };
                        if let Some(command) = command {
                            this.command(command, window, cx);
                            if index == 0 {
                                this.command(Command::BrowseRepository, window, cx);
                            }
                        }
                    }
                },
            ));
        }
    }
    pub(crate) fn sync_live_controls(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self
            .core
            .layer_name_draft
            .as_ref()
            .map(|d| d.name.as_str())
            .unwrap_or_default();
        if self.layer_name_input.read(cx).value().as_ref() != name {
            let name = name.to_string();
            self.layer_name_input
                .update(cx, |state, cx| state.set_value(name, window, cx));
        }
        let connection = &self.core.connection;
        if self.repository_input.read(cx).value().as_ref() != connection.repository {
            let value = connection.repository.clone();
            self.repository_input
                .update(cx, |state, cx| state.set_value(value, window, cx));
        }
        for (index, (mut items, value)) in [
            (
                connection.repositories.clone(),
                connection.repository.clone(),
            ),
            (connection.branches.clone(), connection.branch.clone()),
            (connection.files.clone(), connection.file.clone()),
            (
                connection
                    .workflows
                    .iter()
                    .map(|w| w.path.clone())
                    .collect(),
                connection.workflow.clone(),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            if !value.is_empty() && !items.contains(&value) {
                items.push(value.clone());
            }
            let state = &self.repository_choices[index];
            state.update(cx, |state, cx| {
                state.set_items(SearchableVec::new(items), window, cx);
                if !value.is_empty() {
                    state.set_selected_value(&value, window, cx);
                }
            });
        }
        let fields = self
            .core
            .editor
            .as_ref()
            .map(|draft| draft.fields.clone())
            .unwrap_or_default();
        for field in fields {
            if let Kind::Choice(choices) = field.kind {
                if !self.definition_choices.contains_key(&field.id) {
                    let state = cx.new(|cx| {
                        SelectState::new(SearchableVec::new(choices.clone()), None, window, cx)
                            .searchable(true)
                    });
                    let id = field.id.clone();
                    self._subscriptions.push(cx.subscribe_in(
                        &state,
                        window,
                        move |this: &mut Self, _, event, window, cx| {
                            if let SelectEvent::Confirm(Some(value)) = event
                                && this
                                    .core
                                    .editor
                                    .as_ref()
                                    .is_some_and(|d| d.get(&id) != *value)
                            {
                                this.command(
                                    Command::SetEditorField {
                                        field: id.clone(),
                                        value: value.clone(),
                                    },
                                    window,
                                    cx,
                                );
                            }
                        },
                    ));
                    self.definition_choices.insert(field.id.clone(), state);
                }
                self.definition_choices[&field.id].update(cx, |state, cx| {
                    state.set_items(SearchableVec::new(choices), window, cx);
                    state.set_selected_value(&field.value, window, cx);
                });
            } else {
                if !self.definition_inputs.contains_key(&field.id) {
                    let state = cx.new(|cx| InputState::new(window, cx));
                    let id = field.id.clone();
                    self._subscriptions.push(cx.subscribe_in(
                        &state,
                        window,
                        move |this: &mut Self, input, event, window, cx| {
                            if matches!(event, InputEvent::Change) {
                                let value = input.read(cx).value().to_string();
                                if this
                                    .core
                                    .editor
                                    .as_ref()
                                    .is_some_and(|d| d.get(&id) != value)
                                {
                                    this.command(
                                        Command::SetEditorField {
                                            field: id.clone(),
                                            value,
                                        },
                                        window,
                                        cx,
                                    );
                                }
                            }
                            if matches!(event, InputEvent::PressEnter { .. }) {
                                this.command(Command::ApplyDefinition, window, cx);
                            }
                        },
                    ));
                    self.definition_inputs.insert(field.id.clone(), state);
                }
                if self.definition_inputs[&field.id].read(cx).value().as_ref() != field.value {
                    self.definition_inputs[&field.id]
                        .update(cx, |state, cx| state.set_value(field.value, window, cx));
                }
            }
        }
    }
    pub(crate) fn browse_path(
        &mut self,
        action: PathAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: matches!(action, PathAction::Open),
            directories: !matches!(action, PathAction::Open),
            multiple: false,
            prompt: Some(
                match action {
                    PathAction::Download => "Choose firmware download folder",
                    PathAction::SaveLocal { .. } => {
                        "Choose parent folder for a new local configuration copy"
                    }
                    PathAction::Open => "Open ZMK keymap file",
                    PathAction::OpenFolder => "Open ZMK configuration folder",
                }
                .into(),
            ),
        });
        cx.spawn_in(window, async move |weak, cx| {
            let result = receiver.await;
            let _ = weak.update_in(cx, |this, window, cx| match result {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        this.command(
                            match action {
                                PathAction::Download => Command::DownloadArtifacts(path),
                                PathAction::SaveLocal { build } => Command::SaveLocalCopy {
                                    directory: path,
                                    build,
                                },
                                PathAction::Open | PathAction::OpenFolder => {
                                    Command::OpenLocal(path)
                                }
                            },
                            window,
                            cx,
                        );
                    }
                }
                Ok(Err(error)) => {
                    this.core.status = format!("File picker: {error}");
                    cx.notify();
                }
                _ => {}
            });
        })
        .detach();
    }
    pub(crate) fn save_primary(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.build_setup_prompted = false;
        if self
            .core
            .project
            .zmk
            .as_ref()
            .is_some_and(|d| matches!(d.origin, keyforge_core::zmk::Origin::GitHub { .. }))
        {
            self.browse_path(PathAction::SaveLocal { build: true }, window, cx);
        } else {
            self.command(Command::SaveAndBuild, window, cx);
        }
    }
    pub(crate) fn begin_layer_rename(
        &mut self,
        layer: keyforge_core::LayerId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.core.workspace.focus_mode {
            self.command(Command::ToggleFocusMode, window, cx);
        }
        if !self.core.workspace.visible(keyforge_core::Pane::Left) {
            self.command(Command::TogglePane(keyforge_core::Pane::Left), window, cx);
        }
        self.command(Command::BeginLayerRename(layer), window, cx);
        if self
            .core
            .layer_name_draft
            .as_ref()
            .is_some_and(|d| d.layer == layer)
        {
            self.layer_name_input
                .update(cx, |state, cx| state.focus(window, cx));
            window.dispatch_action(Box::new(gpui_component::input::SelectAll), cx);
        }
    }
}

impl Keyforge {
    pub(crate) fn request_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.closing_prompt || self.closing_storage {
            return false;
        }
        if self.core.connection.busy {
            self.core.status = "Wait for the current repository operation before closing.".into();
            cx.notify();
            return false;
        }
        if matches!(
            self.core.build_environment,
            keyforge_core::build_environment::State::Installing { .. }
        ) {
            self.core.status = "Wait for build environment setup before closing.".into();
            cx.notify();
            return false;
        }
        if self.core.build.status.active() {
            self.core.status = "Cancel the running build before closing Keyforge.".into();
            cx.notify();
            return false;
        }
        self.core.poll();
        let mut storage_error = self.core.session_cache_error().map(str::to_string);
        match self.core.finish_pending_storage() {
            Ok(Some(completion)) => {
                self.closing_storage = true;
                self.core.status = "Remembering your keymap before closing…".into();
                cx.notify();
                let finished = cx
                    .background_executor()
                    .spawn(async move { completion.recv() });
                cx.spawn_in(window, async move |weak, cx| {
                    let _ = finished.await;
                    let _ = weak.update_in(cx, |this, window, cx| {
                        this.closing_storage = false;
                        this.core.poll();
                        if this.request_close(window, cx) {
                            window.remove_window();
                        } else {
                            cx.notify();
                        }
                    });
                })
                .detach();
                return false;
            }
            Err(error) => storage_error = Some(error.to_string()),
            Ok(None) => {}
        }
        let unsaved = self.core.has_changes() || self.core.has_unapplied_draft();
        if !unsaved && storage_error.is_none() {
            return true;
        }
        self.closing_prompt = true;
        let detail = if let Some(error) = &storage_error {
            format!(
                "{error}\n\nYou may need to open your configuration again after restarting.{}",
                if unsaved {
                    " Unsaved edits will also be discarded."
                } else {
                    ""
                }
            )
        } else {
            "Keep editing to save your changes, or discard them and close Keyforge.".into()
        };
        let response = window.prompt(
            PromptLevel::Warning,
            if storage_error.is_some() {
                "Keymap could not be remembered"
            } else {
                "Discard unsaved changes?"
            },
            Some(&detail),
            &[
                "Keep editing",
                if storage_error.is_some() {
                    "Close anyway"
                } else {
                    "Discard and close"
                },
            ],
            cx,
        );
        cx.spawn_in(window, async move |weak, cx| {
            let answer = response.await;
            let _ = weak.update_in(cx, |this, window, cx| {
                this.closing_prompt = false;
                if matches!(answer, Ok(1)) {
                    window.remove_window();
                } else {
                    cx.notify();
                }
            });
        })
        .detach();
        false
    }
}
