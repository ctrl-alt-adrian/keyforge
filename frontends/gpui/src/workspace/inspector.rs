use crate::{app::Keyforge, components::*};
use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme, IconName};
use keyforge_core::{Command, Destination, Pane};

impl Keyforge {
    pub(crate) fn inspector(&self, width: f32, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut panel = div()
            .id("inspector")
            .w(px(width))
            .h_full()
            .flex_shrink_0()
            .overflow_y_scroll()
            .p_6()
            .bg(cx.theme().sidebar)
            .border_l_1()
            .border_color(cx.theme().sidebar_border)
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .mb_5()
                    .child(caption(
                        if self.core.destination == Destination::Keymap {
                            "KEY INSPECTOR"
                        } else {
                            "INSPECTOR"
                        },
                        cx,
                    ))
                    .child(self.icon_button(
                        "close-inspector",
                        IconName::Close,
                        "Close inspector",
                        Command::TogglePane(Pane::Right),
                        cx,
                    )),
            );
        if self.core.destination == Destination::Combos {
            return panel.child(self.definition_form(cx));
        }
        if self.core.destination != Destination::Keymap {
            if let Some(draft) = &self.core.editor
                && keyforge_core::live::section(self.core.destination) == Some(draft.section)
            {
                return panel
                    .child(caption(draft.section.title(), cx))
                    .child(div().mt_4().font_weight(FontWeight::SEMIBOLD).child(
                        if draft.get("name").is_empty() {
                            draft.node.clone().unwrap_or_default()
                        } else {
                            draft.get("name").to_string()
                        },
                    ))
                    .child(
                        div()
                            .mt_4()
                            .text_size(px(13.))
                            .text_color(cx.theme().muted_foreground)
                            .child(draft.section.description()),
                    )
                    .child(div().mt_4().text_size(px(12.)).child(if draft.dirty() {
                        "Invalid draft retained in this editor"
                    } else {
                        "Edits are included in Save and build"
                    }));
            }
            let description = if self.core.destination == Destination::Repository {
                "Open a ZMK configuration repository or a local keymap.".into()
            } else if self.core.destination == Destination::Settings {
                "Preferences affect this device. Configuration edits are preserved.".into()
            } else if let Some(section) = keyforge_core::live::section(self.core.destination) {
                format!("Select a {} to inspect its properties.", section.singular())
            } else {
                "Select an object to inspect its properties.".into()
            };
            return panel.child(
                div()
                    .text_size(px(13.))
                    .text_color(cx.theme().muted_foreground)
                    .child(description),
            );
        }

        let (Some(key), Some(binding)) = (self.core.selected_key, self.core.selected_binding())
        else {
            return panel.child(
                div()
                    .py_8()
                    .text_size(px(14.))
                    .text_color(cx.theme().muted_foreground)
                    .child("Select a key on the canvas to inspect its binding."),
            );
        };
        panel = panel
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .mb_5()
                    .child(key_preview(
                        binding,
                        true,
                        &self.core.project.keymap.layers,
                        &self.hold_tap_behaviors,
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_size(px(17.))
                                    .child(format!("Position {}", key.0)),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!(
                                        "{} layer",
                                        self.core.project.keymap.layers[self.core.active_layer.0]
                                            .name
                                    )),
                            ),
                    ),
            )
            .child(divider(cx))
            .child(caption("Behavior", cx).mt_4().mb_2())
            .child(self.shared_binding_editor(keyforge_core::editing::BindingTarget::Key, cx))
            .child(div().flex_1().min_h(px(24.)))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child(if self.core.has_changes() {
                        "Changes staged"
                    } else {
                        "Keymap workspace"
                    }),
            )
            .child(
                div()
                    .mt_1()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child("Save and build to update firmware."),
            );
        panel
    }
}
