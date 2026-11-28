//! Opt-in development view: `--features gallery -- --gallery`.
use crate::{app::Keyforge, components::*};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, Disableable, IconName,
    button::{Button, ButtonVariants},
    input::Input,
    select::Select,
};
use keyforge_core::{Binding, BuildStatus, Command, KeyCode, KeyPosition, workspace::Appearance};

impl Keyforge {
    pub(crate) fn gallery_view(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id("component-gallery")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_8()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .text_size(px(26.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Keyforge components"),
            )
            .child("Development gallery · production controls, theme and application state")
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(self.button(
                        "gallery-dark",
                        "Dark",
                        Command::SetAppearance(Appearance::Dark),
                        cx,
                    ))
                    .child(self.button(
                        "gallery-light",
                        "Light",
                        Command::SetAppearance(Appearance::Light),
                        cx,
                    ))
                    .child(self.button(
                        "gallery-system",
                        "System",
                        Command::SetAppearance(Appearance::System),
                        cx,
                    )),
            )
            .child(divider(cx))
            .child(caption(
                "ACTIONS · HOVER / PRESS / TAB TO INSPECT FOCUS",
                cx,
            ))
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(
                        self.button("gallery-primary", "Primary", Command::ValidateDocument, cx)
                            .primary(),
                    )
                    .child(self.button(
                        "gallery-secondary",
                        "Secondary",
                        Command::ValidateDocument,
                        cx,
                    ))
                    .child(
                        Button::new("gallery-disabled")
                            .label("Disabled")
                            .disabled(true),
                    )
                    .child(
                        Button::new("gallery-tooltip")
                            .ghost()
                            .icon(IconName::Info)
                            .tooltip("Tooltips use the native component library"),
                    ),
            )
            .child(caption("INPUT AND SEARCHABLE BEHAVIOR SELECTOR", cx))
            .child(self.button(
                "gallery-inspect",
                "Inspect sample key",
                Command::SelectKey(Some(KeyPosition(16))),
                cx,
            ))
            .child(div().w(px(320.)).child(Input::new(&self.inputs[1])))
            .child(
                div()
                    .w(px(320.))
                    .h(px(36.))
                    .child(Select::new(&self.behavior_select).bg(cx.theme().secondary)),
            )
            .child(self.layer_controls(cx))
            .child(caption("KEYCAPS · SAME INSPECTOR PREVIEW COMPONENT", cx))
            .child(
                div().flex().gap_3().children(
                    [
                        (Binding::KeyPress(KeyCode("F".into())), false),
                        (
                            Binding::ModTap {
                                hold: KeyCode("LSHFT".into()),
                                tap: KeyCode("F".into()),
                            },
                            true,
                        ),
                        (Binding::Transparent, false),
                        (Binding::None, false),
                        (Binding::MomentaryLayer(keyforge_core::LayerId(1)), false),
                    ]
                    .map(|(binding, selected)| {
                        key_preview(
                            &binding,
                            selected,
                            &self.core.project.keymap.layers,
                            &self.hold_tap_behaviors,
                            cx,
                        )
                    }),
                ),
            )
            .child(caption("BINDING AND VALIDATION", cx))
            .child(binding_preview(
                self.core
                    .draft
                    .as_ref()
                    .map(|d| d.preview())
                    .unwrap_or_else(|| "Select the sample key above".into()),
                cx,
            ))
            .child(
                div()
                    .text_color(cx.theme().danger_foreground)
                    .child("Error: unknown keycode. Choose a supported keycode."),
            )
            .child(
                div()
                    .text_color(cx.theme().warning_foreground)
                    .child("Warning: this position is disabled."),
            )
            .child(caption("BUILD STATES", cx))
            .child(
                div().flex().gap_5().children(
                    [
                        BuildStatus::Idle,
                        BuildStatus::Queued,
                        BuildStatus::Running,
                        BuildStatus::Successful,
                        BuildStatus::Failed,
                        BuildStatus::Cancelled,
                    ]
                    .map(|s| build_status(s, cx)),
                ),
            )
            .child(caption("LIVE WORKSPACE COMPONENTS", cx))
            .child(
                div()
                    .h(px(620.))
                    .flex()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(self.sidebar(208., cx))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(crate::keyboard::view(self, cx))
                            .child(self.output(180., cx)),
                    )
                    .child(self.inspector(320., cx)),
            )
    }
}
