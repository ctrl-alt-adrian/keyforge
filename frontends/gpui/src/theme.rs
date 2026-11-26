//! Keyforge v0.3 values configure the component library's single active Theme.
use gpui::*;
use gpui_component::{Theme, ThemeMode};
use keyforge_core::{design, workspace::Appearance};

pub fn load_fonts(cx: &mut App) {
    if let Err(error) = cx
        .text_system()
        .add_fonts(vec![design::INTER.into(), design::MONO.into()])
    {
        eprintln!("Unable to load embedded fonts; using platform fallbacks: {error}");
    }
}

pub fn configure(preference: Appearance, window: Option<&mut Window>, cx: &mut App) {
    let mode = match preference {
        Appearance::Dark => ThemeMode::Dark,
        Appearance::Light => ThemeMode::Light,
        Appearance::System => window
            .as_ref()
            .map(|w| w.appearance())
            .unwrap_or_else(|| cx.window_appearance())
            .into(),
    };
    Theme::change(mode, None, cx);
    let t = Theme::global_mut(cx);
    let [
        canvas,
        surface,
        raised,
        border,
        divider,
        accent,
        text,
        muted,
        subtle,
        success,
        warning,
        error,
        selected,
        green,
        amber,
        red,
    ] = if mode == ThemeMode::Dark {
        [
            0x14161a, 0x1b1e24, 0x282c34, 0x414752, 0x30343c, 0x8daafa, 0xe1e7f1, 0x939aa8,
            0x6d7788, 0x85cbb0, 0xe8b777, 0xeb8589, 0x303e58, 0x273d35, 0x3d3427, 0x402d32,
        ]
    } else {
        [
            0xf7f8fa, 0xffffff, 0xedf0f5, 0xcdd3de, 0xe1e5ec, 0x345fc5, 0x202938, 0x566378,
            0x69778b, 0x17694a, 0x8b530d, 0xb53d47, 0xe3ebff, 0xe4f3eb, 0xfff0d8, 0xfbe8e9,
        ]
    };
    t.font_family = "Inter Variable".into();
    t.mono_font_family = "JetBrains Mono".into();
    t.font_size = px(14.);
    t.mono_font_size = px(13.);
    t.radius = px(6.);
    t.shadow = false;
    t.background = rgb(canvas).into();
    t.foreground = rgb(text).into();
    t.sidebar = rgb(surface).into();
    t.sidebar_foreground = rgb(text).into();
    t.sidebar_border = rgb(divider).into();
    t.sidebar_accent = rgb(selected).into();
    t.sidebar_accent_foreground = rgb(accent).into();
    t.secondary = rgb(raised).into();
    t.secondary_foreground = rgb(text).into();
    t.secondary_hover = rgb(selected).into();
    t.secondary_active = rgb(selected).into();
    t.border = rgb(border).into();
    t.input = rgb(border).into(); // Component's `input` means border, not background.
    t.muted = rgb(raised).into();
    t.muted_foreground = rgb(muted).into();
    t.accent = rgb(selected).into();
    t.accent_foreground = rgb(accent).into();
    t.primary = rgb(accent).into();
    t.primary_hover = rgb(accent).into();
    t.primary_active = rgb(accent).into();
    t.primary_foreground = if mode.is_dark() {
        rgb(canvas)
    } else {
        rgb(surface)
    }
    .into();
    t.ring = rgb(accent).into();
    t.selection = rgb(selected).into();
    t.popover = rgb(surface).into();
    t.popover_foreground = rgb(text).into();
    t.list = rgb(surface).into();
    t.list_hover = rgb(raised).into();
    t.list_active = rgb(selected).into();
    t.list_active_border = rgb(accent).into();
    t.tab_bar = rgb(surface).into();
    t.tab = rgb(surface).into();
    t.tab_active = rgb(selected).into();
    t.tab_foreground = rgb(muted).into();
    t.tab_active_foreground = rgb(accent).into();
    t.success = rgb(green).into();
    t.success_foreground = rgb(success).into();
    t.warning = rgb(amber).into();
    t.warning_foreground = rgb(warning).into();
    t.danger = rgb(red).into();
    t.danger_foreground = rgb(error).into();
    t.scrollbar_thumb = rgb(subtle).into();
    t.scrollbar_thumb_hover = rgb(muted).into();
    t.switch = rgb(border).into();
    t.switch_thumb = rgb(surface).into();
    if let Some(window) = window {
        window.refresh();
    }
}
