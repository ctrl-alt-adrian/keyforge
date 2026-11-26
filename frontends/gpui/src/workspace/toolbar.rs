use crate::app::Keyforge;
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, Disableable, IconName, Selectable, button::Button, button::ButtonVariants,
};
use keyforge_core::{Command, Destination, Pane};

impl Keyforge {
    pub(crate) fn toolbar(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let compact = window.viewport_size().width < px(1150.);
        let changes = self.core.pending_count();
        let loaded = self.core.project.zmk.is_some();
        let local = self
            .core
            .project
            .zmk
            .as_ref()
            .is_some_and(|d| matches!(d.origin, keyforge_core::zmk::Origin::Local { .. }));
        let repo = &self.core.project.repository;
        let mut bar = div()
            .h(px(64.))
            .flex_shrink_0()
            .px_5()
            .flex()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().sidebar_border)
            .bg(cx.theme().sidebar)
            .child(
                div()
                    .size(px(28.))
                    .rounded(px(6.))
                    .bg(cx.theme().primary)
                    .text_color(cx.theme().primary_foreground)
                    .flex()
                    .items_center()
                    .justify_center()
                    .font_weight(FontWeight::BOLD)
                    .text_size(px(19.))
                    .child("K"),
            )
            .when(!compact, |el| {
                el.child(
                    div()
                        .ml_1()
                        .mr_6()
                        .text_size(px(20.))
                        .font_weight(FontWeight::BOLD)
                        .child("Keyforge"),
                )
            })
            .child(
                div()
                    .h(px(36.))
                    .w(px(1.))
                    .mx_2()
                    .bg(cx.theme().sidebar_border),
            )
            .child(
                Button::new("open-configuration-toolbar")
                    .ghost()
                    .h(px(44.))
                    .max_w(px(if compact { 260. } else { 600. }))
                    .overflow_hidden()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_start()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(if loaded {
                                        if local {
                                            self.core.project.name.clone()
                                        } else {
                                            format!("{} / {}", repo.owner, repo.name)
                                        }
                                    } else {
                                        "No configuration open".into()
                                    }),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(if loaded {
                                        if local {
                                            "Local keymap".into()
                                        } else {
                                            format!("{} · {}", repo.branch, self.core.project.name)
                                        }
                                    } else {
                                        "Connect a repository or open a local file".into()
                                    }),
                            ),
                    )
                    .tooltip("Open a different configuration")
                    .on_click(cx.listener(|this, _, w, cx| {
                        this.command(Command::Navigate(Destination::Repository), w, cx)
                    })),
            )
            .when(changes > 0 && !compact, |el| {
                el.child(
                    div()
                        .ml_4()
                        .px_3()
                        .py_1()
                        .rounded(px(5.))
                        .bg(cx.theme().warning)
                        .text_color(cx.theme().warning_foreground)
                        .text_size(px(11.))
                        .child(format!("{changes} unsaved")),
                )
            })
            .child(div().flex_1());
        for (pane, id, icon, label) in [
            (
                Pane::Left,
                "toggle-left",
                IconName::PanelLeft,
                "Toggle workspace · Ctrl/Cmd+B",
            ),
            (
                Pane::Bottom,
                "toggle-bottom",
                IconName::PanelBottom,
                "Toggle output · Ctrl/Cmd+J",
            ),
            (
                Pane::Right,
                "toggle-right",
                IconName::PanelRight,
                "Toggle inspector · Ctrl/Cmd+Shift+B",
            ),
        ] {
            bar = bar.child(
                self.icon_button(id, icon, label, Command::TogglePane(pane), cx)
                    .selected(self.core.workspace.visible(pane)),
            );
        }
        bar = bar.when(loaded, |bar| {
            bar.child(
                self.icon_button(
                    "reload-source",
                    IconName::Redo,
                    "Reload keymap from its source",
                    Command::ReloadSource,
                    cx,
                )
                .disabled(self.core.connection.busy),
            )
        });
        bar.child(
            self.icon_button(
                "focus",
                IconName::Maximize,
                "Focus mode · Ctrl/Cmd+Shift+F",
                Command::ToggleFocusMode,
                cx,
            )
            .selected(self.core.workspace.focus_mode),
        )
        .child(
            self.icon_button(
                "settings",
                IconName::Settings,
                "Preferences · Ctrl/Cmd+,",
                Command::Navigate(Destination::Settings),
                cx,
            )
            .selected(self.core.destination == Destination::Settings),
        )
        .when(loaded && !compact, |bar| {
            bar.child(
                Button::new("toolbar-save-copy")
                    .ghost()
                    .label("Save copy…")
                    .tooltip("Save a local configuration copy · Ctrl/Cmd+Shift+S")
                    .disabled(self.core.connection.busy || self.core.build.status.active())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.browse_path(
                            crate::editor_controls::PathAction::SaveLocal { build: false },
                            window,
                            cx,
                        )
                    })),
            )
        })
        .child(
            Button::new("save-build")
                .label(if self.core.build.status.active() {
                    "Cancel build"
                } else {
                    "Save and build"
                })
                .on_click(cx.listener(|this, _, window, cx| {
                    if this.core.build.status.active() {
                        this.command(Command::CancelBuild, window, cx);
                    } else {
                        this.save_primary(window, cx);
                    }
                }))
                .disabled(!loaded || self.core.connection.busy)
                .primary()
                .text_color(cx.theme().primary_foreground)
                .font_weight(FontWeight::SEMIBOLD)
                .ml_2()
                .tooltip("Save the configuration locally and build on this machine · Ctrl/Cmd+S"),
        )
    }
    pub(crate) fn status_bar(&self, cx: &App) -> Div {
        div()
            .h(px(28.))
            .flex_shrink_0()
            .px_4()
            .border_t_1()
            .border_color(cx.theme().sidebar_border)
            .bg(cx.theme().sidebar)
            .text_size(px(11.))
            .text_color(cx.theme().muted_foreground)
            .flex()
            .items_center()
            .gap_4()
            .child(div().size(px(6.)).rounded_full().bg(
                if self.core.build.status == keyforge_core::BuildStatus::Failed {
                    cx.theme().danger_foreground
                } else if self.core.build.status.active() {
                    cx.theme().primary
                } else if self.core.build_environment.ready() {
                    cx.theme().success_foreground
                } else {
                    cx.theme().warning_foreground
                },
            ))
            .child(if self.core.project.zmk.is_some() {
                if local_source(self) {
                    "Local keymap"
                } else {
                    "GitHub configuration"
                }
            } else {
                "No configuration open"
            })
            .child(self.build_indicator(cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(self.core.status.clone()),
            )
            .child(format!(
                "{} changes · {} problems",
                self.core.pending_count(),
                self.core.problem_count()
            ))
    }
}

fn local_source(app: &Keyforge) -> bool {
    app.core
        .project
        .zmk
        .as_ref()
        .is_some_and(|d| matches!(d.origin, keyforge_core::zmk::Origin::Local { .. }))
}
