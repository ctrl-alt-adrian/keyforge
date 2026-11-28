//! One contextual editor for key assignments, combo results, sequence actions
//! and sensor bindings. UI state contains disclosure/focus, never a document.
use crate::{app::Keyforge, components::*};
use gpui::{prelude::*, *};
use gpui_component::button::ButtonVariants;
use gpui_component::{
    ActiveTheme, Disableable, IconName, Selectable, button::Button, input::Input, select::Select,
};
use keyforge_core::{
    Command,
    bindings::{self, ParameterKind},
    editing::{BindingEdit, BindingTarget},
};

impl Keyforge {
    pub(crate) fn activate_binding(
        &mut self,
        target: BindingTarget,
        action: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.binding_target != target && self.core.nested_draft_invalid() {
            self.core.status =
                "Finish or discard the invalid action before opening another editor".into();
            cx.notify();
            return;
        }
        if self.binding_target != target {
            self.binding_definition_preview = false;
        }
        self.binding_target = target;
        self.expanded_action = action;
        self.binding_advanced = false;
        self.sync_inputs(window, cx);
        cx.notify();
    }
    pub(crate) fn shared_binding_editor(
        &self,
        target: BindingTarget,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut view = div().flex().flex_col().gap_3().min_w_0().flex_shrink_0();
        let Some(form) = self.core.assignment_form(&target) else {
            return view.child("No action assigned.");
        };
        let catalog = &self.binding_catalog;
        let spec = catalog.iter().find(|s| s.reference == form.reference);
        let active = self.binding_target == target;
        if active && !self.binding_advanced {
            view = view.child(
                div().h(px(36.)).flex_shrink_0().child(
                    Select::new(&self.behavior_select)
                        .search_placeholder("Find a behavior…")
                        .bg(cx.theme().secondary),
                ),
            );
        } else {
            let next = target.clone();
            view = view.child(
                Button::new(SharedString::from(format!("behavior-{target:?}")))
                    .label(
                        spec.map(|spec| spec.title())
                            .unwrap_or_else(|| form.reference.clone()),
                    )
                    .w_full()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.activate_binding(next.clone(), None, window, cx)
                    })),
            );
        }
        if let Some(spec) = spec {
            view = view.child(
                div()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child(spec.description.clone()),
            );
            let fields = if matches!(target, BindingTarget::Child { .. }) {
                vec![]
            } else {
                spec.fields(&form.arguments)
            };
            if !fields.is_empty() {
                view = view.child(caption("ACTIONS", cx).mt_2());
            }
            for index in spec
                .presentation_order(&form.arguments)
                .into_iter()
                .filter(|index| *index < fields.len())
            {
                let field = &fields[index];
                let expanded =
                    active && self.expanded_action == Some(index) && !self.binding_advanced;
                let value = form.arguments.get(index).map(String::as_str).unwrap_or("");
                let summary = if value.is_empty() {
                    "Not set".into()
                } else {
                    bindings::summary(value, &field.kind, &self.core.project.keymap.layers)
                };
                let kind = match field.kind {
                    ParameterKind::KeyCode => "Keycode",
                    ParameterKind::Layer => "Layer",
                    ParameterKind::Choice(_) => "Action",
                    ParameterKind::Expression => "Value",
                };
                let role = if field.label.to_lowercase().starts_with("tap") {
                    "Tap"
                } else if field.label.to_lowercase().starts_with("hold") {
                    "Hold"
                } else if field.label == "Key code" {
                    "Press"
                } else {
                    &field.label
                };
                let title = format!("{role} · {kind}");
                let next = target.clone();
                view = view.child(
                    action_row(
                        format!("action-{target:?}-{index}"),
                        title,
                        summary,
                        expanded,
                        cx,
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.activate_binding(
                            next.clone(),
                            if expanded { None } else { Some(index) },
                            window,
                            cx,
                        );
                    })),
                );
                if !expanded {
                    continue;
                }
                let mut editor = div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .px_3()
                    .pb_3()
                    .flex_shrink_0();
                if field.kind == ParameterKind::Expression {
                    editor = editor.child(Input::new(&self.inputs[index]));
                } else {
                    if let Some(picker) = &self.parameter_picker {
                        editor = editor.child(
                            div().h(px(160.)).w_full().flex_shrink_0().child(
                                gpui_component::list::List::new(picker)
                                    .search_placeholder(format!(
                                        "Find {}…",
                                        field.label.to_lowercase()
                                    ))
                                    .h_full()
                                    .w_full(),
                            ),
                        );
                    }
                }
                if field.kind == ParameterKind::KeyCode {
                    let (base, modifiers) = bindings::modifiers(value);
                    let intrinsic = bindings::intrinsic_modifiers(&base);
                    editor = editor.child(caption("Combine with modifiers", cx)).child(
                        div().flex().flex_wrap().gap_1().children(
                            bindings::MODIFIER_FUNCTIONS
                                .into_iter()
                                .map(|(modifier, label)| {
                                    let selected = modifiers.iter().any(|m| m == modifier);
                                    let primary = intrinsic.iter().any(|m| m == modifier);
                                    self.button(
                                        format!("modifier-{target:?}-{index}-{modifier}"),
                                        format!("{}{label}", if selected { "✓ " } else { "" }),
                                        Command::EditBinding {
                                            target: target.clone(),
                                            edit: BindingEdit::Modifier {
                                                index,
                                                modifier: modifier.into(),
                                            },
                                        },
                                        cx,
                                    )
                                    .h(px(32.))
                                    .min_w(px(56.))
                                    .px_2()
                                    .text_size(px(11.))
                                    .with_variant(gpui_component::button::ButtonVariant::Secondary)
                                    .border_1()
                                    .border_color(if selected {
                                        cx.theme().primary
                                    } else {
                                        cx.theme().border
                                    })
                                    .selected(selected)
                                    .disabled(primary)
                                    .tooltip(if primary {
                                        "Already part of the primary keycode"
                                    } else {
                                        "Combine with this action's keycode"
                                    })
                                }),
                        ),
                    );
                }
                view = view.child(editor);
            }
        } else {
            view = view.child(
                div()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child("Parameters could not be resolved. The original binding is preserved."),
            );
        }
        if let Some(preview) = &self.binding_preview {
            view = view.child(caption("SHARED DEFINITION", cx).mt_2());
            let automatic = spec.is_some_and(|spec| spec.parameters.is_empty());
            if automatic || self.binding_definition_preview {
                for line in preview.lines.iter().take(6) {
                    view = view.child(
                        div()
                            .text_size(px(12.))
                            .text_color(cx.theme().muted_foreground)
                            .child(line.clone()),
                    );
                }
            } else {
                view = view.child(
                    disclosure("view-shared-definition", "View definition", false).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.binding_definition_preview = true;
                            cx.notify();
                        }),
                    ),
                );
            }
            view = view.child(
                self.button(
                    format!("definition-link-{target:?}"),
                    "Edit behavior definition",
                    Command::OpenDefinition(form.reference.clone()),
                    cx,
                )
                .icon(IconName::ArrowRight)
                .justify_start(),
            );
        }
        let raw_open = active && self.binding_advanced;
        let next = target.clone();
        view = view.child(
            disclosure(
                format!("expression-{target:?}"),
                "Binding expression",
                raw_open,
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.activate_binding(next.clone(), None, window, cx);
                this.binding_advanced = !raw_open;
                cx.notify();
            })),
        );
        if raw_open {
            let source = self.core.assignment_source(&target).unwrap_or_default();
            view = view.child(Input::new(&self.raw_binding_input)).child(
                Button::new("copy-expression")
                    .label("Copy expression")
                    .icon(IconName::Copy)
                    .on_click(move |_, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(source.clone()))
                    }),
            );
        }
        let errors = &self.binding_errors;
        for error in errors {
            view = view.child(validation_message(error.clone(), cx));
        }
        if !errors.is_empty() {
            view = view.child(self.button(
                "discard-invalid-assignment",
                "Discard invalid draft",
                Command::DiscardBindingDraft,
                cx,
            ));
        }
        view.on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
            if event.keystroke.key == "escape" {
                this.expanded_action = None;
                this.binding_advanced = false;
                this.focus.focus(window);
                cx.stop_propagation();
                cx.notify();
            }
        }))
    }
}
