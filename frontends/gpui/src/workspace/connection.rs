use crate::{app::Keyforge, components::*};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, Disableable, IconName,
    button::{Button, ButtonVariants},
    input::Input,
    select::Select,
};
use keyforge_core::Command;

impl Keyforge {
    pub(crate) fn connection_screen(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let state = &self.core.connection;
        let mut form=div().max_w(px(660.)).w_full().flex().flex_col().gap_4()
            .child(div().text_size(px(26.)).font_weight(FontWeight::SEMIBOLD).child("Open a keymap"))
            .child(div().text_color(cx.theme().muted_foreground).child("Connect your ZMK configuration repository or open a local .keymap file."))
            .child(div().text_size(px(12.)).text_color(cx.theme().muted_foreground).child("Choose the repository containing your keyboard configuration, rather than the website editor's source code."))
            .child(divider(cx).my_3())
            .child(div().flex().items_center().gap_3()
                .child(caption(state.account.as_ref().map(|s|format!("GITHUB · {s}")).unwrap_or_else(||"GITHUB".into()),cx))
                .child(div().flex_1())
                .child(self.button("connect-github",if state.account.is_some(){"Refresh repositories"}else{"Connect GitHub"},Command::ConnectGitHub,cx).icon(IconName::GitHub).disabled(state.busy)))
            .child(div().text_size(px(12.)).text_color(cx.theme().muted_foreground).child("Uses your GitHub CLI sign-in. If needed, run gh auth login --web once; Keyforge does not store your token."));
        if !state.repositories.is_empty() {
            form = form.child(caption("Your repositories", cx)).child(
                div().h(px(36.)).child(
                    Select::new(&self.repository_choices[0])
                        .search_placeholder("Find a repository…")
                        .disabled(state.busy)
                        .bg(cx.theme().secondary),
                ),
            );
        }
        form = form
            .child(caption("Repository URL or owner/repository", cx))
            .child(Input::new(&self.repository_input).disabled(state.busy))
            .child(
                self.button(
                    "browse-repository",
                    "Load branches and keymaps",
                    Command::BrowseRepository,
                    cx,
                )
                .disabled(state.busy || state.repository.is_empty())
                .w(px(240.))
                .with_variant(gpui_component::button::ButtonVariant::Secondary),
            );
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
        if state.browsed && state.files.is_empty() {
            form = form.child(
                self.button(
                    "find-configurations",
                    "Find configuration repositories",
                    Command::FindConfigurations,
                    cx,
                )
                .icon(IconName::Search)
                .disabled(state.busy)
                .w(px(300.)),
            );
            if state.editor_repository {
                form = form.child(
                    div()
                        .text_size(px(12.))
                        .text_color(cx.theme().muted_foreground)
                        .child("The website reads and writes your configuration in another repository. There is no personal keymap to generate from its application source."),
                );
            }
        }
        for (index, configuration) in state.configurations.iter().enumerate() {
            let repository = configuration.repository.clone();
            form = form.child(
                Button::new(("configuration-suggestion", index))
                    .label(format!(
                        "{} · {} · {} keymap(s)",
                        configuration.repository, configuration.branch, configuration.keymaps
                    ))
                    .icon(IconName::FolderOpen)
                    .w_full()
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
        if !state.branches.is_empty() {
            form = form.child(caption("Branch", cx)).child(
                div().h(px(36.)).child(
                    Select::new(&self.repository_choices[1])
                        .disabled(state.busy)
                        .bg(cx.theme().secondary),
                ),
            );
        }
        if !state.files.is_empty() {
            form = form.child(caption("Keymap", cx)).child(
                div().h(px(36.)).child(
                    Select::new(&self.repository_choices[2])
                        .disabled(state.busy)
                        .bg(cx.theme().secondary),
                ),
            );
        }
        if !state.files.is_empty() && !state.workflows.is_empty() {
            form = form
                .child(caption("Existing firmware workflow", cx))
                .child(
                    div().h(px(36.)).child(
                        Select::new(&self.repository_choices[3])
                            .disabled(state.busy)
                            .bg(cx.theme().secondary),
                    ),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(cx.theme().muted_foreground)
                        .child(
                            state
                                .workflows
                                .iter()
                                .find(|w| w.path == state.workflow)
                                .map(|w| w.name.clone())
                                .unwrap_or_default(),
                        ),
                );
        }
        if !state.files.is_empty() {
            form = form
                .child(caption(
                    format!(
                        "Physical layout: {} · {} keys",
                        self.core.project.keyboard.name,
                        self.core.project.keyboard.keys.len()
                    ),
                    cx,
                ))
                .child(
                    self.button(
                        "open-repository",
                        "Open keymap",
                        Command::OpenRepository,
                        cx,
                    )
                    .primary()
                    .w(px(160.))
                    .disabled(state.busy),
                );
        }
        form=form.child(divider(cx).my_3()).child(caption("LOCAL FILE",cx))
            .child(div().flex().flex_wrap().gap_2()
                .child(Button::new("open-local-folder").label("Open configuration folder…").icon(IconName::FolderOpen).disabled(state.busy).on_click(cx.listener(|this,_,window,cx|this.browse_path(crate::editor_controls::PathAction::OpenFolder,window,cx))))
                .child(Button::new("open-local").label("Open .keymap file…").disabled(state.busy).on_click(cx.listener(|this,_,window,cx|this.browse_path(crate::editor_controls::PathAction::Open,window,cx)))))
            .child(div().text_size(px(12.)).text_color(cx.theme().muted_foreground).child("Choose a .keymap file or its configuration folder. Local builds use Docker or Podman, build.yaml and the West manifest. Firmware stays on this machine."));
        if self.core.project.zmk.is_some() {
            form = form
                .child(divider(cx))
                .child(caption("CURRENT CONFIGURATION", cx))
                .child(
                    Button::new("save-local-copy")
                        .label("Save local configuration copy…")
                        .icon(IconName::FolderOpen)
                        .disabled(state.busy || self.core.build.status.active())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.browse_path(
                                crate::editor_controls::PathAction::SaveLocal { build: false },
                                window,
                                cx,
                            )
                        })),
                );
            let remote = self
                .core
                .project
                .zmk
                .as_ref()
                .is_some_and(|d| matches!(d.origin, keyforge_core::zmk::Origin::GitHub { .. }));
            form = form.child(
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
                form = form.child(self.button("github-save-build","Save and build on GitHub",Command::SaveAndBuildOnGitHub,cx).disabled(state.busy||self.core.build.status.active()))
                .child(div().text_size(px(12.)).text_color(cx.theme().muted_foreground).child("These GitHub actions commit changes to the selected branch. The toolbar's primary action saves a local copy and builds locally."));
            }
        }
        if self.core.has_changes() || self.core.has_unapplied_draft() {
            form=form.child(divider(cx)).child(div().text_color(cx.theme().warning_foreground).child("The current configuration has pending edits. Save them before switching, or explicitly discard them."))
            .child(self.button("discard-edits","Discard pending edits",Command::DiscardChanges,cx).danger().w(px(220.)).disabled(state.busy));
        }
        div()
            .id("connection-screen")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_8()
            .flex()
            .flex_col()
            .items_center()
            .child(form)
    }
}
