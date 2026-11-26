use crate::{app::Keyforge, components::*};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, Disableable, IconName, Selectable, button::ButtonVariants, input::Input,
    select::Select,
};
use keyforge_core::{
    Command,
    editor::{self, Kind, Section},
};

impl Keyforge {
    pub(crate) fn definitions_screen(&self, section: Section, cx: &mut Context<Self>) -> Div {
        let records = editor::entries(&self.core.project, section, self.core.active_layer.0)
            .unwrap_or_default();
        let mut list = div()
            .id("definition-list")
            .w(px(220.))
            .flex_shrink_0()
            .h_full()
            .overflow_y_scroll()
            .pr_4()
            .flex()
            .flex_col()
            .gap_2();
        for record in &records {
            list = list.child(
                self.button(
                    format!("definition-{}", record.id),
                    record.name.clone(),
                    Command::SelectEditor(record.id.clone()),
                    cx,
                )
                .w_full()
                .justify_start()
                .selected(
                    self.core.editor.as_ref().and_then(|d| d.node.as_deref())
                        == Some(record.id.as_str()),
                )
                .tooltip(record.detail.clone()),
            );
        }
        if section != Section::Encoders {
            list = list.child(
                self.button(
                    "new-definition",
                    format!("New {}", section.singular()),
                    Command::NewDefinition,
                    cx,
                )
                .icon(IconName::Plus)
                .mt_3()
                .w_full(),
            );
        }
        let content = if self.core.editor.is_some() {
            self.definition_form(cx).into_any_element()
        } else {
            div().p_6().text_color(cx.theme().muted_foreground).child(if section==Section::Encoders{"No encoder bindings are defined in this keymap. Configured sensor bindings appear here.".into()}else{format!("No {} yet. Create a {} to get started.",section.title().to_lowercase(),section.singular())}).into_any_element()
        };
        let mut screen = div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .p_7()
            .gap_4()
            .child(
                div()
                    .text_size(px(26.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(section.title()),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(cx.theme().muted_foreground)
                    .child(section.description()),
            );
        if section == Section::Encoders {
            screen = screen.child(self.layer_controls(cx));
        }
        if section == Section::Combos {
            screen=screen.child(div().flex().flex_wrap().gap_2().children(records.iter().map(|record|self.button(format!("combo-{}",record.id),record.name.clone(),Command::SelectEditor(record.id.clone()),cx).selected(self.core.editor.as_ref().and_then(|d|d.node.as_deref())==Some(record.id.as_str()))))
                .child(self.button("new-combo","New combo",Command::NewDefinition,cx).icon(IconName::Plus)))
                .child(crate::keyboard::view(self,cx))
                .child(div().text_size(px(12.)).text_color(cx.theme().muted_foreground).child("Click keys to toggle membership in the selected combo. Apply the definition to stage changes."));
            if !self.core.workspace.visible(keyforge_core::Pane::Right) {
                screen = screen.child(div().h(px(260.)).child(self.definition_form(cx)));
            }
        } else {
            screen = screen.child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(list)
                    .child(div().w(px(1.)).h_full().bg(cx.theme().sidebar_border))
                    .child(div().flex_1().min_w_0().pl_6().child(content)),
            );
        }
        screen
    }
    pub(crate) fn definition_form(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut form = div()
            .id("definition-form")
            .size_full()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .pb_6();
        let Some(draft) = &self.core.editor else {
            return form.child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child("Select a definition or create one to edit it."),
            );
        };
        form = form.child(
            div()
                .text_size(px(18.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(if draft.get("name").is_empty() {
                    format!("New {}", draft.section.singular())
                } else {
                    draft.get("name").to_owned()
                }),
        );
        if draft.node.is_some() {
            form = form.child(
                div()
                    .text_size(px(11.))
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        if matches!(draft.section, Section::Macros | Section::Behaviors) {
                            format!("Binding reference: &{}", draft.get("name"))
                        } else {
                            format!("Definition in {}", self.core.project.name)
                        },
                    ),
            );
        }

        for field in &draft.fields {
            if let Some(index) = field
                .id
                .strip_prefix("step-mode-")
                .and_then(|s| s.parse::<usize>().ok())
            {
                form = form.child(divider(cx).mt_3()).child(
                    div()
                        .flex()
                        .items_center()
                        .child(caption(format!("STEP {}", index + 1), cx))
                        .child(div().flex_1())
                        .child(
                            self.button(
                                format!("step-up-{index}"),
                                "",
                                Command::MoveMacroStep { index, delta: -1 },
                                cx,
                            )
                            .icon(IconName::ArrowUp)
                            .tooltip("Move step up")
                            .disabled(index == 0),
                        )
                        .child(
                            self.button(
                                format!("step-down-{index}"),
                                "",
                                Command::MoveMacroStep { index, delta: 1 },
                                cx,
                            )
                            .icon(IconName::ArrowDown)
                            .tooltip("Move step down")
                            .disabled(
                                index + 1
                                    >= draft
                                        .fields
                                        .iter()
                                        .filter(|f| f.id.starts_with("step-mode-"))
                                        .count(),
                            ),
                        )
                        .child(
                            self.button(
                                format!("remove-step-{index}"),
                                "Remove",
                                Command::RemoveMacroStep(index),
                                cx,
                            )
                            .text_size(px(11.)),
                        ),
                );
            }
            form = form.child(caption(field.label.clone(), cx));
            if matches!(field.kind, Kind::Choice(_)) {
                if let Some(state) = self.definition_choices.get(&field.id) {
                    form = form.child(
                        div()
                            .h(px(36.))
                            .flex_shrink_0()
                            .child(Select::new(state).bg(cx.theme().secondary)),
                    );
                }
            } else if let Some(state) = self.definition_inputs.get(&field.id) {
                form = form.child(Input::new(state));
            }
            if matches!(field.kind, Kind::Layer | Kind::Layers) {
                form = form.child(
                    div()
                        .text_size(px(11.))
                        .text_color(cx.theme().muted_foreground)
                        .child(
                            self.core
                                .project
                                .keymap
                                .layers
                                .iter()
                                .enumerate()
                                .map(|(i, l)| format!("{i}: {}", l.name))
                                .collect::<Vec<_>>()
                                .join(" · "),
                        ),
                );
            }
        }
        if draft.section == Section::Encoders
            && draft.fields.iter().any(|f| f.id == "encoder-clockwise")
        {
            form = form
                .child(caption("Generated sensor binding", cx))
                .child(binding_preview(
                    format!(
                        "&inc_dec_kp {} {}",
                        draft.get("encoder-clockwise"),
                        draft.get("encoder-counterclockwise")
                    ),
                    cx,
                ));
        }
        if draft.section == Section::Macros {
            form = form.child(
                self.button(
                    "add-macro-step",
                    "Add sequence step",
                    Command::AddMacroStep,
                    cx,
                )
                .icon(IconName::Plus),
            );
        }
        form=form.child(divider(cx).mt_3()).child(div().flex().flex_wrap().gap_2()
            .child(self.button("apply-definition","Apply changes",Command::ApplyDefinition,cx).primary().disabled(!draft.dirty()))
            .child(self.button("revert-definition",if draft.node.is_none(){"Cancel"}else{"Revert draft"},Command::RevertDefinition,cx)))
            .child(div().text_size(px(12.)).text_color(cx.theme().muted_foreground).child("Apply creates one undoable edit. Save and build saves your configuration and starts a local build."));
        if draft.node.is_some() && draft.section != Section::Encoders {
            form = form.child(
                self.button(
                    "delete-definition",
                    format!("Delete {}", draft.section.singular()),
                    Command::DeleteDefinition,
                    cx,
                )
                .danger()
                .mt_3(),
            );
        }
        form
    }
}
