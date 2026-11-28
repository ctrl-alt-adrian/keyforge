use crate::{app::Keyforge, components::*};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, Disableable, IconName, Selectable,
    button::{Button, ButtonVariants},
    input::Input,
    select::Select,
};
use keyforge_core::{
    Command, LayerId, bindings,
    editing::BindingTarget,
    editor::{Field, Kind, Section},
    zmk,
};

impl Keyforge {
    pub(crate) fn definitions_screen(
        &self,
        section: Section,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let records = &self.definition_records;
        let mut navigation = div()
            .id("definition-list")
            .w(px(200.))
            .flex_shrink_0()
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2()
            .pr_4()
            .when(compact, |view| {
                view.w_full()
                    .h_auto()
                    .max_h(px(120.))
                    .flex_row()
                    .flex_wrap()
                    .pr_0()
            });
        for record in records {
            navigation = navigation.child(
                self.button(
                    format!("definition-{}", record.id),
                    record.name.clone(),
                    Command::SelectEditor(record.id.clone()),
                    cx,
                )
                .w_full()
                .justify_start()
                .selected(
                    self.core
                        .editor
                        .as_ref()
                        .and_then(|draft| draft.node.as_deref())
                        == Some(&record.id),
                )
                .tooltip(record.detail.clone()),
            );
        }
        if section != Section::Encoders {
            navigation = navigation.child(
                self.button(
                    "new-definition",
                    format!("New {}", section.singular()),
                    Command::NewDefinition,
                    cx,
                )
                .icon(IconName::Plus)
                .justify_start()
                .mt_3(),
            );
        }
        let mut screen = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .p_6()
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
        if section == Section::Combos {
            screen = screen
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .children(records.iter().map(|record| {
                            self.button(
                                format!("combo-{}", record.id),
                                record.name.clone(),
                                Command::SelectEditor(record.id.clone()),
                                cx,
                            )
                            .selected(
                                self.core
                                    .editor
                                    .as_ref()
                                    .and_then(|draft| draft.node.as_deref())
                                    == Some(record.id.as_str()),
                            )
                        }))
                        .child(
                            self.button("new-combo", "New combo", Command::NewDefinition, cx)
                                .icon(IconName::Plus),
                        ),
                )
                .child(crate::keyboard::view(self, cx))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(cx.theme().muted_foreground)
                        .child("Click physical keys to add or remove combo positions."),
                );
            if !self.core.workspace.visible(keyforge_core::Pane::Right) {
                screen = screen.child(div().h(px(320.)).min_h_0().child(self.definition_form(cx)));
            }
        } else {
            screen = screen.child(
                div()
                    .flex()
                    .when(compact, |view| view.flex_col())
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .child(navigation)
                    .child(
                        div()
                            .w(px(1.))
                            .h_full()
                            .bg(cx.theme().sidebar_border)
                            .when(compact, |view| view.w_full().h(px(1.)).flex_shrink_0()),
                    )
                    .child(
                        div()
                            .pl_5()
                            .when(compact, |view| view.pl_0().pt_4())
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .child(self.definition_form(cx)),
                    ),
            );
        }
        screen
    }
    pub(crate) fn summarize_binding(
        &self,
        source: &str,
        catalog: &[bindings::BehaviorSpec],
    ) -> String {
        zmk::binding_spans(source)
            .unwrap_or_default()
            .into_iter()
            .map(|span| {
                let source = &source[span];
                let Ok((reference, arguments)) = bindings::split(source) else {
                    return source.to_string();
                };
                let Some(spec) = catalog.iter().find(|spec| spec.reference == reference) else {
                    return source.to_string();
                };
                let values = spec
                    .fields(&arguments)
                    .iter()
                    .zip(&arguments)
                    .map(|(field, value)| {
                        bindings::summary(value, &field.kind, &self.core.project.keymap.layers)
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                if values.is_empty() {
                    spec.name.clone()
                } else if reference == "&kp" {
                    values
                } else {
                    format!("{} · {values}", spec.name)
                }
            })
            .collect::<Vec<_>>()
            .join(" · ")
    }
    fn binding_summary(&self, source: &str) -> String {
        self.definition_summaries
            .get(source)
            .cloned()
            .unwrap_or_else(|| source.to_string())
    }
    fn definition_field(&self, field: &Field, cx: &mut Context<Self>) -> Div {
        let mut row = div()
            .flex()
            .flex_col()
            .gap_2()
            .flex_shrink_0()
            .min_w_0()
            .child(caption(field.label.clone(), cx));
        if field.kind == Kind::Layers {
            row = row.child(div().flex().flex_wrap().gap_2().children(
                field.value.split_whitespace().map(|value| {
                    let layer = value.parse::<usize>().ok();
                    let label = layer
                        .and_then(|i| {
                            self.core
                                .project
                                .keymap
                                .layers
                                .get(i)
                                .map(|l| format!("{i} — {}", l.name))
                        })
                        .unwrap_or_else(|| value.to_string());
                    let mut button = Button::new(SharedString::from(format!(
                        "layer-chip-{}-{value}",
                        field.id
                    )))
                    .label(label)
                    .icon(IconName::Close);
                    if let Some(layer) = layer {
                        let id = field.id.clone();
                        button = button.on_click(cx.listener(move |this, _, window, cx| {
                            this.command(
                                Command::ToggleEditorLayer {
                                    field: id.clone(),
                                    layer: LayerId(layer),
                                },
                                window,
                                cx,
                            )
                        }));
                    }
                    button
                }),
            ));
        }
        if matches!(field.kind, Kind::Choice(_) | Kind::Layer | Kind::Layers) {
            if let Some(state) = self.definition_choices.get(&field.id) {
                row = row.child(
                    div().h(px(36.)).flex_shrink_0().child(
                        Select::new(state)
                            .search_placeholder(if field.kind == Kind::Layers {
                                "Add a layer…"
                            } else {
                                "Search…"
                            })
                            .bg(cx.theme().secondary),
                    ),
                );
            }
        } else if let Some(state) = self.definition_inputs.get(&field.id) {
            row = row.child(Input::new(state));
        }
        row
    }
    fn binding_children(&self, field: &str, references_only: bool, cx: &mut Context<Self>) -> Div {
        let mut result = div().flex().flex_col().gap_3().min_w_0().flex_shrink_0();
        let Some(draft) = &self.core.editor else {
            return result;
        };
        let value = draft.get(field);
        let spans = zmk::binding_spans(value).unwrap_or_default();
        let mut indices = (0..spans.len()).collect::<Vec<_>>();
        if references_only && indices.len() == 2 {
            indices.swap(0, 1);
        }
        for index in indices {
            let target = if references_only {
                BindingTarget::Child {
                    field: field.into(),
                    binding: index,
                }
            } else {
                BindingTarget::Field {
                    field: field.into(),
                    binding: index,
                }
            };
            let source = &value[spans[index].clone()];
            if self.binding_target == target {
                result = result.child(self.shared_binding_editor(target, cx));
            } else {
                let label = if references_only {
                    if index == 0 {
                        "Hold · Child behavior".into()
                    } else {
                        "Tap · Child behavior".into()
                    }
                } else {
                    format!("Action {}", index + 1)
                };
                result = result.child(
                    action_row(
                        format!("child-{field}-{index}"),
                        label,
                        self.binding_summary(source),
                        false,
                        cx,
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.activate_binding(target.clone(), None, window, cx)
                    })),
                );
            }
        }
        result
    }
    fn macro_sequence(&self, cx: &mut Context<Self>) -> Div {
        let mut sequence = div()
            .flex()
            .flex_col()
            .gap_3()
            .min_w_0()
            .flex_shrink_0()
            .child(caption("SEQUENCE", cx));
        let Some(draft) = &self.core.editor else {
            return sequence;
        };
        let count = draft
            .fields
            .iter()
            .filter(|field| field.id.starts_with("step-mode-"))
            .count();
        let mut inherited = "Tap";
        for index in 0..count {
            let mode = draft.get(&format!("step-mode-{index}"));
            let field = format!("step-bindings-{index}");
            let source = draft.get(&field);
            let references = zmk::binding_spans(source).unwrap_or_default();
            let first = references
                .first()
                .map(|span| &source[span.clone()])
                .unwrap_or(source);
            let operation = if mode != "Raw" {
                inherited = mode;
                mode.to_string()
            } else if first == "&macro_press" {
                inherited = "Press";
                "Press".into()
            } else if first == "&macro_release" {
                inherited = "Release";
                "Release".into()
            } else if first == "&macro_tap" {
                inherited = "Tap";
                "Tap".into()
            } else if first.starts_with("&macro_wait_time") || first.starts_with("&macro_tap_time")
            {
                "Timing control".into()
            } else if first.starts_with("&macro_pause_for_release") {
                "Pause until release".into()
            } else if first.starts_with("&macro_param_") {
                "Parameter forwarding".into()
            } else {
                inherited.to_string()
            };
            let open = self.expanded_step == Some(index);
            let target_field = field.clone();
            sequence = sequence.child(
                action_row(
                    format!("sequence-step-{index}"),
                    format!("{:02} · {operation}", index + 1),
                    {
                        let summary = self.binding_summary(source);
                        if summary.is_empty() {
                            operation.clone()
                        } else {
                            summary
                        }
                    },
                    open,
                    cx,
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    if this.core.nested_draft_invalid() && this.expanded_step != Some(index) {
                        this.core.status =
                            "Finish or discard the invalid action before opening another step"
                                .into();
                        cx.notify();
                        return;
                    }
                    this.expanded_step = if open { None } else { Some(index) };
                    this.definition_source = false;
                    if !open {
                        this.activate_binding(
                            BindingTarget::Field {
                                field: target_field.clone(),
                                binding: 0,
                            },
                            None,
                            window,
                            cx,
                        );
                    }
                    cx.notify();
                })),
            );
            if open {
                let mut body = div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .px_3()
                    .pb_3()
                    .flex_shrink_0()
                    .min_w_0();
                if self.definition_source {
                    if let Some(state) = self.definition_inputs.get(&field) {
                        body = body.child(Input::new(state));
                    }
                } else {
                    if mode != "Raw"
                        && let Some(mode) = draft
                            .fields
                            .iter()
                            .find(|field| field.id == format!("step-mode-{index}"))
                    {
                        body = body.child(self.definition_field(mode, cx));
                    }
                    body = body.child(self.binding_children(&field, false, cx));
                }
                body = body
                    .child(
                        disclosure(
                            format!("step-source-{index}"),
                            "Sequence expression",
                            self.definition_source,
                        )
                        .disabled(self.core.nested_draft_invalid())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.definition_source = !this.definition_source;
                            this.sync_live_controls(window, cx);
                            cx.notify();
                        })),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .child(
                                self.button(
                                    format!("step-up-{index}"),
                                    "Up",
                                    Command::MoveMacroStep { index, delta: -1 },
                                    cx,
                                )
                                .disabled(index == 0),
                            )
                            .child(
                                self.button(
                                    format!("step-down-{index}"),
                                    "Down",
                                    Command::MoveMacroStep { index, delta: 1 },
                                    cx,
                                )
                                .disabled(index + 1 == count),
                            )
                            .child(self.button(
                                format!("step-remove-{index}"),
                                "Remove",
                                Command::RemoveMacroStep(index),
                                cx,
                            ))
                            .child(self.button(
                                format!("step-insert-{index}"),
                                "Insert after",
                                Command::InsertMacroStep(index + 1),
                                cx,
                            )),
                    );
                sequence = sequence.child(body);
            }
        }
        sequence.child(
            self.button("add-macro-step", "Add action", Command::AddMacroStep, cx)
                .icon(IconName::Plus)
                .justify_start(),
        )
    }
    pub(crate) fn definition_form(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut form = div()
            .id("definition-form")
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_4()
            .pb_6();
        let Some(draft) = &self.core.editor else {
            return form.child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child("Select a definition or create one to begin."),
            );
        };
        form = form.child(
            div()
                .text_size(px(19.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(if draft.get("name").is_empty() {
                    if draft.section == Section::Encoders {
                        draft.node.clone().unwrap_or_else(|| "Encoder".into())
                    } else {
                        format!("New {}", draft.section.singular())
                    }
                } else {
                    draft.get("name").into()
                }),
        );
        if draft.section == Section::Behaviors || draft.section == Section::Macros {
            form = form.child(
                div()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child("Shared definition · changes affect every referencing binding."),
            );
        }
        if let Some(name) = draft.fields.iter().find(|field| field.id == "name") {
            form = form.child(self.definition_field(name, cx));
        }
        let kind = &self.definition_kind;
        match draft.section {
            Section::Combos => {
                form = form
                    .child(caption("TRIGGER", cx))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .children(draft.positions().iter().map(|position| {
                                let position = *position;
                                self.button(
                                    format!("combo-position-{position}"),
                                    format!("Position {position}"),
                                    Command::ToggleComboPosition(position),
                                    cx,
                                )
                                .icon(IconName::Close)
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                "Click physical keys on the keyboard to add or remove positions.",
                            ),
                    )
                    .child(caption("RESULT", cx))
                    .child(self.shared_binding_editor(
                        BindingTarget::Field {
                            field: "bindings".into(),
                            binding: 0,
                        },
                        cx,
                    ));
            }
            Section::Macros => form = form.child(self.macro_sequence(cx)),
            Section::Behaviors => {
                if let Some(field) = draft
                    .fields
                    .iter()
                    .find(|field| field.id == "behavior-type")
                {
                    form = form.child(self.definition_field(field, cx));
                }
                if draft.fields.iter().any(|field| field.id == "bindings")
                    && !self.definition_source
                {
                    form = form
                        .child(caption("CHILD BEHAVIORS", cx))
                        .child(self.binding_children(
                            "bindings",
                            kind == "zmk,behavior-hold-tap",
                            cx,
                        ));
                }
                if kind == "zmk,behavior-hold-tap" {
                    form = form.child(
                        div()
                            .text_size(px(12.))
                            .text_color(cx.theme().muted_foreground)
                            .child("Keycode arguments belong to each key assignment."),
                    );
                }
                for field in draft.fields.iter().filter(|field| {
                    ["flavor", "tapping-term-ms", "quick-tap-ms", "mods"]
                        .contains(&field.id.as_str())
                }) {
                    form = form.child(self.definition_field(field, cx));
                }
            }
            Section::ConditionalLayers => {
                if let Some(field) = draft.fields.iter().find(|field| field.id == "if-layers") {
                    form = form.child(self.definition_field(field, cx));
                }
                if let Some(field) = draft.fields.iter().find(|field| field.id == "then-layer") {
                    form = form.child(self.definition_field(field, cx));
                }
            }
            Section::Encoders => {
                form = form
                    .child(caption("Layer", cx))
                    .child(self.layer_controls(cx))
                    .child(caption("ROTATION BEHAVIOR", cx))
                    .child(self.shared_binding_editor(BindingTarget::Sensor, cx));
            }
        }
        let label = match draft.section {
            Section::Combos => "Conditions",
            Section::Macros => "Timing defaults",
            _ => "Advanced options",
        };
        if matches!(
            draft.section,
            Section::Combos | Section::Macros | Section::Behaviors
        ) {
            form = form.child(
                disclosure("definition-advanced", label, self.definition_advanced).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.definition_advanced = !this.definition_advanced;
                        cx.notify();
                    }),
                ),
            );
        }
        if self.definition_advanced {
            for field in &draft.fields {
                let visible = match draft.section {
                    Section::Combos => {
                        !["name", "bindings", "key-positions"].contains(&field.id.as_str())
                    }
                    Section::Macros => ["wait-ms", "tap-ms"].contains(&field.id.as_str()),
                    Section::Behaviors => ![
                        "name",
                        "behavior-type",
                        "bindings",
                        "flavor",
                        "tapping-term-ms",
                        "quick-tap-ms",
                        "mods",
                    ]
                    .contains(&field.id.as_str()),
                    _ => false,
                };
                if visible {
                    form = form.child(self.definition_field(field, cx));
                }
            }
            if draft.section == Section::Behaviors
                && draft.fields.iter().any(|field| field.id == "bindings")
            {
                form = form.child(
                    disclosure(
                        "definition-source",
                        "Child binding expressions",
                        self.definition_source,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.definition_source = !this.definition_source;
                        cx.notify();
                    })),
                );
                if self.definition_source
                    && let Some(field) = draft.fields.iter().find(|field| field.id == "bindings")
                {
                    form = form.child(self.definition_field(field, cx));
                }
            }
        }
        if let Some(error) = &draft.error {
            form = form.child(validation_message(error.clone(), cx));
        }
        if draft.dirty() {
            form = form.child(self.button(
                "revert-invalid-definition",
                "Discard invalid draft",
                Command::RevertDefinition,
                cx,
            ));
        }
        if draft.node.is_some() && draft.section != Section::Encoders {
            form = form.child(
                self.button(
                    "delete-definition",
                    format!("Delete {}", draft.section.singular()),
                    Command::DeleteDefinition,
                    cx,
                )
                .danger()
                .justify_start(),
            );
        }
        form.child(
            div()
                .text_size(px(12.))
                .text_color(cx.theme().muted_foreground)
                .child(
                    "Valid edits stage immediately · Save and build persists the configuration.",
                ),
        )
    }
}
