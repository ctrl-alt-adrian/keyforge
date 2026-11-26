use crate::{app::Keyforge, components::*};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, Disableable, IconName,
    button::{Button, ButtonVariants},
    input::Input,
    select::Select,
};
use keyforge_core::{Command, Destination, Pane, Severity};

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
            if let Some(draft) = &self.core.editor {
                return panel
                    .child(caption(draft.section.title(), cx))
                    .child(
                        div()
                            .mt_4()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(draft.get("name").to_string()),
                    )
                    .child(
                        div()
                            .mt_4()
                            .text_size(px(13.))
                            .text_color(cx.theme().muted_foreground)
                            .child(draft.section.description()),
                    )
                    .child(div().mt_4().text_size(px(12.)).child(if draft.dirty() {
                        "Definition draft has unapplied changes"
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

        let (Some(key), Some(draft), Some(binding)) = (
            self.core.selected_key,
            &self.core.draft,
            self.core.selected_binding(),
        ) else {
            return panel.child(
                div()
                    .py_8()
                    .text_size(px(14.))
                    .text_color(cx.theme().muted_foreground)
                    .child("Select a key on the canvas to inspect its binding."),
            );
        };
        let draft_changed = draft.binding().as_ref().ok() != Some(binding);
        let errors = self.core.draft_diagnostics();
        let invalid = errors.iter().any(|d| d.severity == Severity::Error);
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
                        self.core.project.zmk.as_ref(),
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
                                    .text_size(px(16.))
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
            .child(caption("Behavior", cx).mt_5().mb_3())
            .child(
                div().h(px(36.)).flex_shrink_0().child(
                    Select::new(&self.behavior_select)
                        .search_placeholder("Find a behavior…")
                        .bg(cx.theme().secondary),
                ),
            );
        for (index, label) in draft.behavior.parameter_labels().iter().enumerate() {
            if draft.behavior == keyforge_core::Behavior::Custom {
                panel = panel
                    .child(caption(*label, cx).mt_5().mb_3())
                    .child(Input::new(&self.inputs[index]));
                continue;
            }
            panel = panel.child(caption(*label, cx).mt_5().mb_3()).child(
                div().h(px(36.)).flex_shrink_0().child(
                    Select::new(&self.parameter_selects[index])
                        .search_placeholder(format!("Find {}…", label.to_lowercase()))
                        .bg(cx.theme().secondary),
                ),
            );
        }
        panel = panel
            .child(caption("Generated binding", cx).mt_7().mb_3())
            .child(binding_preview(draft.preview(), cx))
            .child(
                div()
                    .mt_3()
                    .text_size(px(13.))
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        draft
                            .binding()
                            .map(|binding| {
                                binding_description(
                                    &binding,
                                    &self.core.project.keymap.layers,
                                    self.core.project.zmk.as_ref(),
                                )
                            })
                            .unwrap_or_else(|_| draft.behavior.description().into()),
                    ),
            );
        for diagnostic in errors {
            panel = panel.child(
                div()
                    .mt_3()
                    .text_size(px(12.))
                    .text_color(if diagnostic.severity == Severity::Error {
                        cx.theme().danger_foreground
                    } else {
                        cx.theme().warning_foreground
                    })
                    .child(format!("{:?}: {}", diagnostic.severity, diagnostic.message)),
            );
        }
        panel
            .child(
                div()
                    .mt_5()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child(if draft_changed {
                        "Draft changed. Apply to stage this edit."
                    } else {
                        "Edits are staged until Save and build."
                    }),
            )
            .child(
                self.button("apply-binding", "Apply binding", Command::ApplyBinding, cx)
                    .disabled(invalid || !draft_changed)
                    .mt_3(),
            )
            .child(
                Button::new("copy-binding")
                    .ghost()
                    .label("Copy binding")
                    .icon(IconName::Copy)
                    .mt_2()
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(draft) = &this.core.draft {
                            cx.write_to_clipboard(ClipboardItem::new_string(draft.preview()));
                        }
                    })),
            )
            .child(divider(cx).mt_4())
            .child(caption("KEY HELP", cx).mt_4().mb_3())
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child(draft.behavior.description()),
            )
    }
}
