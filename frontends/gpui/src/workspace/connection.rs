use crate::{app::Keyforge, components::*};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, Disableable, IconName, Selectable,
    button::{Button, ButtonVariants},
    input::Input,
    select::Select,
};
use keyforge_core::Command;

impl Keyforge {
    pub(crate) fn connection_screen(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let state = &self.core.connection;
        let mut form = div()
            .w_full()
            .max_w(px(660.))
            .min_w_0()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .text_size(px(26.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Open a keymap"),
            )
            .child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child("Choose the configuration you want to edit."),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("open-github-tab")
                            .ghost()
                            .label("GitHub")
                            .selected(!self.opening_local)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.opening_local = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("open-local-tab")
                            .ghost()
                            .label("Local file")
                            .selected(self.opening_local)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.opening_local = true;
                                cx.notify();
                            })),
                    ),
            );
        if self.opening_local {
            form = form.child(caption("LOCAL CONFIGURATION", cx))
                .child(div().text_size(px(13.)).text_color(cx.theme().muted_foreground).child("Open a keymap file or its configuration folder. Local edits save to disk."))
                .child(ActionRow::new(
                    Button::new("open-local-folder").label("Open folder…").icon(IconName::FolderOpen).disabled(state.busy)
                        .on_click(cx.listener(|this, _, window, cx| this.browse_path(crate::editor_controls::PathAction::OpenFolder, window, cx))),
                    Button::new("open-local-file").label("Open keymap…").disabled(state.busy)
                        .on_click(cx.listener(|this, _, window, cx| this.browse_path(crate::editor_controls::PathAction::Open, window, cx)))));
        } else if state.account.is_none() {
            form = form
                .child(caption("GITHUB", cx))
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(cx.theme().muted_foreground)
                        .child(
                            "Connect your GitHub CLI account to choose a configuration repository.",
                        ),
                )
                .child(
                    self.button(
                        "connect-github",
                        "Connect GitHub",
                        Command::ConnectGitHub,
                        cx,
                    )
                    .icon(IconName::GitHub)
                    .disabled(state.busy),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(cx.theme().muted_foreground)
                        .child("You can open a local file without signing in."),
                );
        } else {
            form = form.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(caption(
                        format!("GITHUB · {}", state.account.as_deref().unwrap_or_default()),
                        cx,
                    ))
                    .child(div().flex_1())
                    .child(
                        self.button("refresh-account", "Refresh", Command::ConnectGitHub, cx)
                            .disabled(state.busy),
                    ),
            );
            if !state.repositories.is_empty() && !self.opening_manual_repository {
                form = form.child(caption("Repository", cx)).child(
                    div().h(px(36.)).child(
                        Select::new(&self.repository_choices[0])
                            .search_placeholder("Find a configuration repository…")
                            .disabled(state.busy)
                            .bg(cx.theme().secondary),
                    ),
                );
            } else {
                form = form
                    .child(caption("Repository URL or owner/repository", cx))
                    .child(Input::new(&self.repository_input).disabled(state.busy))
                    .child(
                        self.button(
                            "browse-repository",
                            "Load keymaps",
                            Command::BrowseRepository,
                            cx,
                        )
                        .disabled(state.busy || state.repository.is_empty()),
                    );
            }
            if !state.repositories.is_empty() {
                form = form.child(
                    disclosure(
                        "manual-repository",
                        "Use a repository URL",
                        self.opening_manual_repository,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.opening_manual_repository = !this.opening_manual_repository;
                        cx.notify();
                    })),
                );
            }
            if !state.branches.is_empty() {
                form = form.child(caption("Branch", cx)).child(
                    div().h(px(36.)).child(
                        Select::new(&self.repository_choices[1])
                            .search_placeholder("Find a branch…")
                            .disabled(state.busy)
                            .bg(cx.theme().secondary),
                    ),
                );
            }
            if !state.files.is_empty() {
                form = form
                    .child(caption("Keymap", cx))
                    .child(
                        div().h(px(36.)).child(
                            Select::new(&self.repository_choices[2])
                                .search_placeholder("Find a keymap…")
                                .disabled(state.busy)
                                .bg(cx.theme().secondary),
                        ),
                    )
                    .child(
                        self.button(
                            "open-repository",
                            "Open keymap",
                            Command::OpenRepository,
                            cx,
                        )
                        .primary()
                        .disabled(state.busy),
                    );
            }
            if state.browsed && state.files.is_empty() {
                form = form.child(
                    self.button(
                        "find-configurations",
                        "Find configuration repositories",
                        Command::FindConfigurations,
                        cx,
                    )
                    .icon(IconName::Search)
                    .disabled(state.busy),
                );
                for (index, configuration) in state.configurations.iter().enumerate() {
                    let repository = configuration.repository.clone();
                    form = form.child(
                        Button::new(("configuration-suggestion", index))
                            .label(format!(
                                "{} · {} keymap(s)",
                                repository, configuration.keymaps
                            ))
                            .disabled(state.busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.command(
                                    Command::SetConnection {
                                        field: "repository".into(),
                                        value: repository.clone(),
                                    },
                                    window,
                                    cx,
                                );
                                this.command(Command::BrowseRepository, window, cx);
                            })),
                    );
                }
            }
        }
        if state.busy {
            form = form.child(
                div()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child("Loading configuration…"),
            );
        }
        if !state.message.is_empty() {
            form = form.child(
                div()
                    .p_3()
                    .rounded(px(6.))
                    .bg(cx.theme().secondary)
                    .text_size(px(13.))
                    .child(state.message.clone()),
            );
        }
        if let Some(document) = &self.core.project.zmk {
            form = form.child(divider(cx)).child(
                disclosure(
                    "current-configuration",
                    "Current configuration",
                    self.opening_current,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.opening_current = !this.opening_current;
                    cx.notify();
                })),
            );
            if self.opening_current {
                let remote = matches!(document.origin, keyforge_core::zmk::Origin::GitHub { .. });
                form = form
                    .child(
                        Button::new("save-local-copy")
                            .label("Save local copy…")
                            .icon(IconName::FolderOpen)
                            .disabled(state.busy || self.core.build.status.active())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.browse_path(
                                    crate::editor_controls::PathAction::SaveLocal { build: false },
                                    window,
                                    cx,
                                )
                            })),
                    )
                    .child(
                        self.button(
                            "save-only",
                            if remote {
                                "Save to GitHub"
                            } else {
                                "Save file"
                            },
                            Command::SaveDocument,
                            cx,
                        )
                        .disabled(state.busy),
                    );
                if remote {
                    form = form.child(div().text_size(px(12.)).text_color(cx.theme().muted_foreground).child("Save to GitHub writes the selected branch. Firmware builds run locally."));
                }
            }
        }
        if self.core.has_changes() || self.core.has_unapplied_draft() {
            form = form.child(divider(cx)).child(div().text_color(cx.theme().warning_foreground).child("The current configuration has pending edits. Save or explicitly discard them before opening another keymap."))
                .child(self.button("discard-edits", "Discard pending edits", Command::DiscardChanges, cx).danger().disabled(state.busy));
        }
        div()
            .id("connection-screen")
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_y_scroll()
            .p_6()
            .flex()
            .flex_col()
            .items_center()
            .child(form)
    }
}
