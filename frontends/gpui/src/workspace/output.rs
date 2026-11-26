use crate::{app::Keyforge, components::*};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, Disableable, IconName, Selectable,
    button::{Button, ButtonVariants},
};
use keyforge_core::{BottomTab, Command};

impl Keyforge {
    pub(crate) fn output(&self, height: f32, cx: &mut Context<Self>) -> Div {
        let preferences = &self.core.workspace.preferences;
        let sizes = self
            .core
            .workspace
            .rendered(preferences.window_width, preferences.window_height - 92.);
        let compact = preferences.window_width - sizes.left - sizes.right < 600.;
        let mut tabs = div()
            .flex()
            .gap_1()
            .items_center()
            .px_4()
            .h(px(42.))
            .flex_shrink_0()
            .border_b_1()
            .border_color(cx.theme().sidebar_border);
        for tab in BottomTab::ALL {
            tabs = tabs.child(
                self.button(
                    format!("output-{tab:?}"),
                    match tab {
                        BottomTab::BuildOutput => {
                            if compact { "Build" } else { "Build output" }.into()
                        }
                        BottomTab::Problems => {
                            format!("Problems ({})", self.core.problem_count())
                        }
                        BottomTab::Changes => {
                            if compact {
                                "Changes".into()
                            } else {
                                format!("Changes ({})", self.core.pending_count())
                            }
                        }
                    },
                    Command::SelectBottomTab(tab),
                    cx,
                )
                .selected(self.core.workspace.preferences.bottom_tab == tab),
            );
        }
        tabs = tabs
            .child(div().flex_1())
            .when(!compact, |el| el.child(self.build_indicator(cx)))
            .child(self.icon_button(
                "close-output",
                IconName::Close,
                "Close output",
                Command::TogglePane(keyforge_core::Pane::Bottom),
                cx,
            ));
        let mut body = div()
            .id("output-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px_5()
            .py_3()
            .flex()
            .flex_col()
            .gap_2()
            .text_size(px(12.));
        match self.core.workspace.preferences.bottom_tab {
            BottomTab::BuildOutput => {
                body = body.overflow_hidden().child(
                    list(
                        self.build_output_list.clone(),
                        cx.processor(move |this, index, _, cx| {
                            this.build_output_row(index, compact, cx).into_any_element()
                        }),
                    )
                    .flex_1()
                    .min_h_0()
                    .w_full(),
                );
            }
            BottomTab::Problems => {
                if self.core.problem_count() == 0 {
                    body = body.child("No problems.");
                }
                for problem in &self.core.build.problems {
                    body = body.child(
                        div()
                            .flex()
                            .gap_2()
                            .items_start()
                            .text_color(if problem.severity == keyforge_core::Severity::Error {
                                cx.theme().danger_foreground
                            } else {
                                cx.theme().warning_foreground
                            })
                            .child(gpui_component::Icon::new(IconName::TriangleAlert))
                            .child(div().min_w_0().child(format!(
                                "{:?} · Build · {}",
                                problem.severity, problem.message
                            ))),
                    );
                }
                for (i, d) in self.core.diagnostics.iter().enumerate() {
                    body = body.child(
                        self.button(
                            format!("problem-{i}"),
                            format!(
                                "{:?} · {} / {} · {}",
                                d.severity,
                                self.core.project.keymap.layers[d.layer.0].name,
                                d.key.0,
                                d.message
                            ),
                            Command::RevealDiagnostic {
                                layer: d.layer,
                                key: d.key,
                            },
                            cx,
                        )
                        .justify_start(),
                    );
                }
            }
            BottomTab::Changes => {
                let changes = self.core.changes();
                if self.core.has_changes() && changes.is_empty() {
                    body = body.child("Configuration definitions have pending changes.");
                }
                if changes.is_empty() && !self.core.has_changes() {
                    body = body.child("No pending changes.");
                }
                for change in changes {
                    body = body.child(
                        div()
                            .font_family(cx.theme().mono_font_family.clone())
                            .child(format!(
                                "{} / {}   {} → {}",
                                self.core.project.keymap.layers[change.layer.0].name,
                                change.key.0,
                                change.before.zmk(),
                                change.after.zmk()
                            )),
                    );
                }
            }
        }
        div()
            .h(px(height))
            .w_full()
            .flex()
            .flex_col()
            .bg(cx.theme().sidebar)
            .border_t_1()
            .border_color(cx.theme().sidebar_border)
            .child(tabs)
            .child(body)
    }

    fn build_output_row(&self, index: usize, compact: bool, cx: &mut Context<Self>) -> Div {
        if index == 0 {
            return div().flex().items_center().gap_4().child(caption(
                if compact || self.core.project.zmk.is_none() {
                    "BUILD OUTPUT"
                } else {
                    if self.core.project.zmk.as_ref().is_some_and(|d| {
                        matches!(d.origin, keyforge_core::zmk::Origin::GitHub { .. })
                    }) {
                        "REPOSITORY FIRMWARE WORKFLOW"
                    } else {
                        "LOCAL FIRMWARE BUILD"
                    }
                },
                cx,
            ));
        }
        if let Some(line) = self.core.build.output.get(index - 1) {
            return div()
                .w_full()
                .min_h(px(18.))
                .pb_1()
                .font_family(cx.theme().mono_font_family.clone())
                .text_size(px(12.))
                .whitespace_normal()
                .text_color(match keyforge_core::build_line_severity(line) {
                    Some(keyforge_core::Severity::Error) => cx.theme().danger_foreground,
                    Some(keyforge_core::Severity::Warning) => cx.theme().warning_foreground,
                    None => cx.theme().foreground,
                })
                .child(line.clone());
        }
        let mut footer = div().w_full().flex().flex_col().gap_2().py_2();
        for artifact in &self.core.build.artifacts {
            footer = footer.child(
                div()
                    .text_color(cx.theme().success_foreground)
                    .child(format!("{} · {} bytes", artifact.name, artifact.bytes)),
            );
        }
        if let Some(directory) = &self.core.local_firmware {
            let directory = directory.clone();
            footer = footer.child(
                Button::new("open-local-firmware")
                    .label("Open firmware folder")
                    .icon(IconName::FolderOpen)
                    .on_click(move |_, _, cx| cx.reveal_path(&directory)),
            );
        }
        if let Some(run) = &self.core.workflow_run {
            let url = run.url.clone();
            footer = footer.child(
                Button::new("open-workflow")
                    .ghost()
                    .label("View workflow on GitHub")
                    .icon(IconName::ExternalLink)
                    .on_click(move |_, _, cx| cx.open_url(&url)),
            );
            if !self.core.build.artifacts.is_empty() {
                footer = footer.child(
                    Button::new("download-firmware")
                        .label("Download firmware artifacts…")
                        .icon(IconName::ArrowDown)
                        .disabled(self.core.connection.busy)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.browse_path(
                                crate::editor_controls::PathAction::Download,
                                window,
                                cx,
                            )
                        })),
                );
            }
        }
        let setting_up = matches!(
            self.core.build_environment,
            keyforge_core::build_environment::State::Installing { .. }
        );
        if self.core.build.status.active() || setting_up {
            let indicator = if self.core.workspace.preferences.reduced_motion {
                gpui_component::Icon::new(IconName::LoaderCircle).into_any_element()
            } else {
                gpui_component::spinner::Spinner::new().into_any_element()
            };
            footer = footer.child(div().flex().items_center().gap_2().child(indicator).child(
                if setting_up {
                    "Setting up local builds…"
                } else if self.core.workflow_run.is_some()
                    || self.core.project.zmk.as_ref().is_some_and(|d| {
                        matches!(d.origin, keyforge_core::zmk::Origin::GitHub { .. })
                    })
                {
                    "GitHub workflow in progress…"
                } else {
                    "Local firmware build in progress…"
                },
            ));
        }
        footer
    }
}
