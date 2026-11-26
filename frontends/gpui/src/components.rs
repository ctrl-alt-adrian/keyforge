use gpui::{prelude::*, *};
use gpui_component::ActiveTheme;
use keyforge_core::{Binding, BuildStatus};

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
pub fn binding_preview(text: String, cx: &App) -> Div {
    div()
        .w_full()
        .px_3()
        .py_2()
        .rounded(px(5.))
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .font_family(cx.theme().mono_font_family.clone())
        .text_size(px(13.))
        .child(text)
}
pub fn binding_description(
    binding: &Binding,
    layers: &[keyforge_core::Layer],
    document: Option<&keyforge_core::zmk::Document>,
) -> String {
    if matches!(binding, Binding::Custom(_))
        && let Some(behaviors) = document.and_then(|doc| doc.hold_tap_behaviors().ok())
        && keyforge_core::zmk::resolved_hold_tap(binding, &behaviors).is_some()
    {
        return crate::keycap::for_binding(binding, layers, &behaviors).description;
    }
    let layer_name = |id: keyforge_core::LayerId| {
        layers
            .get(id.0)
            .map(|layer| layer.name.clone())
            .unwrap_or_else(|| format!("layer {}", id.0))
    };
    match binding {
        Binding::ModTap { hold, tap } => {
            let modifier = match hold.0.as_str() {
                "LSHFT" => "Left Shift",
                "RSHFT" => "Right Shift",
                "LCTRL" => "Left Control",
                "RCTRL" => "Right Control",
                "LALT" => "Left Alt",
                "RALT" => "Right Alt",
                "LGUI" => "Left GUI",
                "RGUI" => "Right GUI",
                other => other,
            };
            format!("Tap for {}. Hold for {modifier}.", tap.0)
        }
        Binding::LayerTap { layer, tap } => format!(
            "Tap for {}. Hold to activate {}.",
            tap.0,
            layer_name(*layer)
        ),
        Binding::MomentaryLayer(layer) => format!("Activate {} while held.", layer_name(*layer)),
        Binding::KeyPress(code) => format!("Send {} while held.", code.0),
        _ => binding.behavior().description().into(),
    }
}
pub fn key_preview(
    binding: &Binding,
    selected: bool,
    layers: &[keyforge_core::Layer],
    document: Option<&keyforge_core::zmk::Document>,
    cx: &App,
) -> Div {
    let behaviors = document
        .and_then(|doc| doc.hold_tap_behaviors().ok())
        .unwrap_or_default();
    let label = crate::keycap::for_binding(binding, layers, &behaviors);
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
