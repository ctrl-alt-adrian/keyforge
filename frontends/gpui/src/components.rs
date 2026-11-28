use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme,
    button::{Button, ButtonVariant, ButtonVariants},
};
use keyforge_core::{Binding, BuildStatus};

#[derive(IntoElement)]
pub struct ExpandableActionRow {
    button: Button,
    trigger: SharedString,
    value: SharedString,
    expanded: bool,
}
impl ExpandableActionRow {
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.button = self.button.on_click(handler);
        self
    }
}
impl RenderOnce for ExpandableActionRow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .relative()
            .w_full()
            .min_w_0()
            .h(px(64.))
            .flex_shrink_0()
            .child(self.button.absolute().top_0().left_0().w_full().h_full())
            .child(
                div()
                    .absolute()
                    .left(px(12.))
                    .right(px(12.))
                    .top(px(8.))
                    .bottom(px(8.))
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(self.trigger),
                            )
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .text_color(cx.theme().foreground)
                                    .truncate()
                                    .child(self.value),
                            ),
                    )
                    .child(
                        gpui_component::Icon::new(if self.expanded {
                            gpui_component::IconName::ChevronUp
                        } else {
                            gpui_component::IconName::ChevronDown
                        })
                        .flex_shrink_0(),
                    ),
            )
    }
}
pub fn action_row(
    id: impl Into<SharedString>,
    trigger: impl Into<SharedString>,
    value: impl Into<SharedString>,
    expanded: bool,
    cx: &App,
) -> ExpandableActionRow {
    let value = value.into();
    let button = Button::new(id.into())
        .with_variant(ButtonVariant::Secondary)
        .w_full()
        .h(px(64.))
        .border_1()
        .border_color(if expanded {
            cx.theme().primary
        } else {
            cx.theme().border
        })
        .when(value.len() > 32, |button| button.tooltip(value.clone()));
    ExpandableActionRow {
        button,
        trigger: trigger.into(),
        value,
        expanded,
    }
}
pub fn disclosure(
    id: impl Into<SharedString>,
    title: impl Into<SharedString>,
    expanded: bool,
) -> Button {
    Button::new(id.into())
        .ghost()
        .label(title)
        .h(px(36.))
        .w_full()
        .justify_start()
        .icon(if expanded {
            gpui_component::IconName::ChevronUp
        } else {
            gpui_component::IconName::ChevronDown
        })
}
pub fn validation_message(message: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .text_size(px(12.))
        .text_color(cx.theme().danger_foreground)
        .child(message.into())
}

pub struct Confirmation<'a> {
    pub title: &'a str,
    pub detail: &'a str,
    pub confirm: &'a str,
    pub cancel: &'a str,
    pub destructive: bool,
}
/// The same themed, in-window confirmation surface for lifecycle safeguards.
pub fn confirmation(
    dialog: gpui_component::dialog::Dialog,
    options: Confirmation<'_>,
    cx: &App,
) -> gpui_component::dialog::Dialog {
    use gpui_component::dialog::DialogButtonProps;
    dialog
        .confirm()
        .w(px(480.))
        .bg(cx.theme().popover)
        .text_color(cx.theme().foreground)
        .border_1()
        .border_color(cx.theme().border)
        .rounded(px(6.))
        .title(
            div()
                .text_size(px(19.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(options.title.to_string()),
        )
        .child(
            div()
                .w_full()
                .text_size(px(13.))
                .whitespace_normal()
                .text_color(cx.theme().muted_foreground)
                .child(options.detail.to_string()),
        )
        .button_props(
            DialogButtonProps::default()
                .ok_text(options.confirm.to_string())
                .cancel_text(options.cancel.to_string())
                .ok_variant(if options.destructive {
                    ButtonVariant::Danger
                } else {
                    ButtonVariant::Primary
                })
                .cancel_variant(ButtonVariant::Secondary),
        )
}

/// A primary and secondary action with equal widths and a shared button size.
/// Callers provide labels, availability and handlers; layout stays consistent.
#[derive(IntoElement)]
pub struct ActionRow {
    primary: Button,
    secondary: Button,
}
impl ActionRow {
    pub fn new(primary: Button, secondary: Button) -> Self {
        Self { primary, secondary }
    }
}
impl RenderOnce for ActionRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let size = |button: Button| {
            button
                .flex_1()
                .min_w_0()
                .h(px(36.))
                .px_3()
                .text_size(px(13.))
        };
        div()
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .flex_shrink_0()
            .child(size(self.primary.primary()))
            .child(size(self.secondary.with_variant(ButtonVariant::Secondary)))
    }
}

pub fn caption(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(cx.theme().muted_foreground)
        .child(text.into())
}
pub fn divider(cx: &App) -> Div {
    div()
        .h(px(1.))
        .w_full()
        .flex_shrink_0()
        .bg(cx.theme().sidebar_border)
}
#[cfg(feature = "gallery")]
pub fn binding_preview(text: String, cx: &App) -> Div {
    div()
        .w_full()
        .p_3()
        .rounded(px(6.))
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .font_family(cx.theme().mono_font_family.clone())
        .text_size(px(13.))
        .child(text)
}
pub fn key_preview(
    binding: &Binding,
    selected: bool,
    layers: &[keyforge_core::Layer],
    behaviors: &std::collections::BTreeMap<String, [String; 2]>,
    cx: &App,
) -> Div {
    let label = crate::keycap::for_binding(binding, layers, behaviors);
    let primary = if let Some(path) = label.primary.icon {
        svg().path(path).size(px(22.)).into_any_element()
    } else {
        div()
            .text_size(px(18.))
            .max_w(px(46.))
            .whitespace_nowrap()
            .text_ellipsis()
            .overflow_hidden()
            .child(label.primary.text)
            .into_any_element()
    };
    div()
        .w(px(54.))
        .h(px(56.))
        .flex_shrink_0()
        .rounded(px(7.))
        .border_1()
        .border_color(if selected {
            cx.theme().primary
        } else if matches!(
            binding,
            Binding::MomentaryLayer(_) | Binding::LayerTap { .. }
        ) {
            cx.theme().success_foreground
        } else {
            cx.theme().border
        })
        .bg(if selected {
            cx.theme().selection
        } else if matches!(
            binding,
            Binding::MomentaryLayer(_) | Binding::LayerTap { .. }
        ) {
            cx.theme().success
        } else {
            cx.theme().secondary
        })
        .flex()
        .flex_col()
        .justify_center()
        .items_center()
        .gap_1()
        .child(
            div()
                .text_size(px(18.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(if matches!(binding, Binding::Transparent | Binding::None) {
                    cx.theme().muted_foreground
                } else {
                    cx.theme().foreground
                })
                .child(primary),
        )
        .child(
            div()
                .text_size(px(9.))
                .text_color(cx.theme().muted_foreground)
                .max_w(px(46.))
                .whitespace_nowrap()
                .text_ellipsis()
                .overflow_hidden()
                .child(label.secondary.text),
        )
}
pub fn build_status(status: BuildStatus, cx: &App) -> Div {
    let (text, color) = match status {
        BuildStatus::Idle => ("Ready", cx.theme().muted_foreground),
        BuildStatus::Queued => ("Queued", cx.theme().warning_foreground),
        BuildStatus::Running => ("Building", cx.theme().primary),
        BuildStatus::Successful => ("Successful", cx.theme().success_foreground),
        BuildStatus::Failed => ("Failed", cx.theme().danger_foreground),
        BuildStatus::Cancelled => ("Cancelled", cx.theme().muted_foreground),
    };
    div().text_size(px(12.)).text_color(color).child(text)
}

/// Application preferences use the same label/description/control composition.
pub fn setting_row(title: &str, description: &str, control: impl IntoElement, cx: &App) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_3()
        .py_4()
        .border_b_1()
        .border_color(cx.theme().sidebar_border)
        .flex_shrink_0()
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title.to_string()),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(cx.theme().muted_foreground)
                .child(description.to_string()),
        )
        .child(control)
}
