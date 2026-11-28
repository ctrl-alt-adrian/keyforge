//! Compact, readable legends over shared binding semantics. Icons are embedded
//! Lucide SVGs; raw bindings remain available in tooltips and the inspector.
use gpui::*;
use keyforge_core::{Binding, Layer, zmk};
use std::collections::BTreeMap;

pub struct Legend {
    pub text: String,
    pub icon: Option<&'static str>,
    pub name: String,
}
impl Legend {
    fn text(value: impl Into<String>) -> Self {
        let text = value.into();
        Self {
            name: text.clone(),
            text,
            icon: None,
        }
    }
    fn icon(path: &'static str, name: &str) -> Self {
        Self {
            text: String::new(),
            icon: Some(path),
            name: name.into(),
        }
    }
    pub fn empty(&self) -> bool {
        self.text.is_empty() && self.icon.is_none()
    }
}
pub struct KeycapLabel {
    pub primary: Legend,
    pub secondary: Legend,
    pub description: String,
}
fn label(primary: Legend, secondary: Legend) -> KeycapLabel {
    let description = if secondary.empty() {
        primary.name.clone()
    } else {
        format!("{} · {}", primary.name, secondary.name)
    };
    KeycapLabel {
        primary,
        secondary,
        description,
    }
}
pub fn for_binding(
    binding: &Binding,
    layers: &[Layer],
    behaviors: &BTreeMap<String, [String; 2]>,
) -> KeycapLabel {
    if let Some((hold, tap)) = zmk::resolved_hold_tap(binding, behaviors) {
        let tap = simple(&tap, layers);
        let hold_label = match &hold {
            Binding::KeyPress(code) => Legend::text(
                modifier(&code.0).unwrap_or_else(|| format!("Hold {}", keycode(&code.0).name)),
            ),
            Binding::MomentaryLayer(layer) => Legend::text(format!("L{} hold", layer.0)),
            _ => Legend::text(hold.zmk()),
        };
        return KeycapLabel {
            description: format!(
                "Tap {} · hold {}",
                tap.primary.name,
                hold_label.name.trim_start_matches("Hold ")
            ),
            primary: tap.primary,
            secondary: hold_label,
        };
    }
    simple(binding, layers)
}
fn simple(binding: &Binding, layers: &[Layer]) -> KeycapLabel {
    let blank = || Legend::text("");
    match binding {
        Binding::KeyPress(code) => {
            let (base, modifiers) = keyforge_core::bindings::modifiers(&code.0);
            let modifier_labels = modifiers
                .iter()
                .filter_map(|function| {
                    keyforge_core::bindings::MODIFIER_FUNCTIONS
                        .iter()
                        .find(|(name, _)| *name == function)
                        .map(|(_, label)| *label)
                })
                .collect::<Vec<_>>()
                .join("+");
            label(keycode(&base), Legend::text(modifier_labels))
        }
        Binding::MomentaryLayer(layer) => {
            let name = layers
                .get(layer.0)
                .map(|l| l.name.clone())
                .unwrap_or_else(|| format!("Layer {}", layer.0));
            let icon = match name.to_ascii_lowercase().as_str() {
                "lower" | "base" | "default" => "icons/arrow-down.svg",
                "raise" | "upper" => "icons/arrow-up.svg",
                "system" => "icons/settings.svg",
                _ => "keycaps/layers.svg",
            };
            let mut result = label(Legend::icon(icon, "Layer"), Legend::text(&name));
            result.description = format!("Activate {name} while held");
            result
        }
        Binding::Transparent => label(
            Legend::icon(
                "keycaps/arrow-down-to-line.svg",
                "Transparent: use an underlying layer",
            ),
            blank(),
        ),
        Binding::None => label(Legend::icon("icons/minus.svg", "No binding"), blank()),
        Binding::Custom(value) => {
            let parts: Vec<_> = value.split_whitespace().collect();
            match parts.as_slice() {
                ["&bt", "BT_SEL", slot] => label(
                    Legend::icon("keycaps/bluetooth.svg", "Bluetooth profile"),
                    Legend::text(*slot),
                ),
                ["&bt", "BT_CLR"] => label(
                    Legend::icon("keycaps/bluetooth.svg", "Clear Bluetooth bonds"),
                    Legend::text("Clear"),
                ),
                ["&out", "OUT_BLE"] => label(
                    Legend::icon("keycaps/bluetooth.svg", "Bluetooth output"),
                    Legend::text("Output"),
                ),
                ["&out", "OUT_USB"] => label(
                    Legend::icon("keycaps/usb.svg", "USB output"),
                    Legend::text("Output"),
                ),
                ["&out", "OUT_TOG"] => label(
                    Legend::icon("keycaps/repeat-2.svg", "Toggle output"),
                    Legend::text("Output"),
                ),
                [name, params @ ..] => label(
                    Legend::text(name.trim_start_matches('&')),
                    Legend::text(params.join(" ")),
                ),
                [] => label(Legend::text("Custom"), blank()),
            }
        }
        _ => label(
            Legend::text(binding.label()),
            Legend::text(binding.secondary_label()),
        ),
    }
}
fn modifier(code: &str) -> Option<String> {
    Some(
        match code {
            "LCTRL" | "LEFT_CONTROL" => "L Ctrl",
            "RCTRL" | "RIGHT_CONTROL" => "R Ctrl",
            "LALT" | "LEFT_ALT" => "L Alt",
            "RALT" | "RIGHT_ALT" => "R Alt",
            "LGUI" | "LEFT_GUI" | "LEFT_WIN" => {
                if cfg!(target_os = "macos") {
                    "L Cmd"
                } else if cfg!(target_os = "windows") {
                    "L Win"
                } else {
                    "L Super"
                }
            }
            "RGUI" | "RIGHT_GUI" | "RIGHT_WIN" => {
                if cfg!(target_os = "macos") {
                    "R Cmd"
                } else if cfg!(target_os = "windows") {
                    "R Win"
                } else {
                    "R Super"
                }
            }
            "LSHFT" | "LEFT_SHIFT" => "L Shift",
            "RSHFT" | "RIGHT_SHIFT" => "R Shift",
            _ => return None,
        }
        .into(),
    )
}
fn keycode(code: &str) -> Legend {
    let icon = match code {
        "RET" | "RETURN" | "ENTER" => Some(("keycaps/corner-down-left.svg", "Enter")),
        "SPACE" => Some(("keycaps/space.svg", "Space")),
        "BSPC" | "BACKSPACE" => Some(("icons/delete.svg", "Backspace")),
        "LSHFT" | "RSHFT" | "LEFT_SHIFT" | "RIGHT_SHIFT" => {
            Some(("keycaps/arrow-big-up.svg", "Shift"))
        }
        "LGUI" | "RGUI" | "LEFT_GUI" | "RIGHT_GUI" | "LEFT_WIN" | "RIGHT_WIN" => {
            Some(("keycaps/command.svg", "GUI modifier"))
        }
        "LEFT" => Some(("icons/arrow-left.svg", "Left")),
        "RIGHT" => Some(("icons/arrow-right.svg", "Right")),
        "UP" => Some(("icons/arrow-up.svg", "Up")),
        "DOWN" => Some(("icons/arrow-down.svg", "Down")),
        "C_VOL_DN" | "C_VOLUME_DOWN" => Some(("keycaps/volume-1.svg", "Volume down")),
        "C_VOL_UP" | "C_VOLUME_UP" => Some(("keycaps/volume-2.svg", "Volume up")),
        "C_MUTE" => Some(("keycaps/volume-x.svg", "Mute")),
        "C_PREV" | "C_PREVIOUS" => Some(("keycaps/skip-back.svg", "Previous track")),
        "C_NEXT" => Some(("keycaps/skip-forward.svg", "Next track")),
        "C_PLAY_PAUSE" | "C_PP" => Some(("keycaps/play.svg", "Play / pause")),
        "K_POWER" | "C_POWER" => Some(("keycaps/power.svg", "Power")),
        "K_SCREENSAVER" => Some(("keycaps/lock-keyhole.svg", "Lock / screensaver")),
        "PRINTSCREEN" | "PSCRN" => Some(("keycaps/camera.svg", "Print screen")),
        "C_AL_CALCULATOR" | "C_AL_CALC" => Some(("keycaps/calculator.svg", "Calculator")),
        "C_AC_PRINT" => Some(("keycaps/printer.svg", "Print")),
        "C_AL_EMAIL" | "C_AL_MAIL" => Some(("keycaps/mail.svg", "Email")),
        "C_AL_FILE_BROWSER" | "C_AL_FILES" => Some(("icons/folder-open.svg", "Files")),
        "C_AL_CALENDAR" | "C_AL_CAL" => Some(("keycaps/calendar.svg", "Calendar")),
        "C_AL_DOCUMENTS" | "C_AL_DOCS" => Some(("keycaps/file-text.svg", "Documents")),
        "C_AL_KEYBOARD_LAYOUT" => Some(("keycaps/keyboard.svg", "Keyboard layout")),
        "C_AL_MY_COMPUTER" | "C_AL_MY_COMPU" => Some(("keycaps/monitor.svg", "My computer")),
        _ => None,
    };
    if let Some((path, name)) = icon {
        return Legend::icon(path, name);
    }
    let text = match code {
        "ESC" | "ESCAPE" => "Esc",
        "TAB" => "Tab",
        "CAPS" | "CAPSLOCK" => "Caps",
        "LCTRL" | "RCTRL" | "LEFT_CONTROL" | "RIGHT_CONTROL" => "Ctrl",
        "LALT" | "RALT" | "LEFT_ALT" | "RIGHT_ALT" => "Alt",
        "SEMI" | "SEMICOLON" => ";",
        "SQT" | "SINGLE_QUOTE" => "'",
        "DQT" | "DOUBLE_QUOTES" => "\"",
        "COMMA" => ",",
        "DOT" | "PERIOD" => ".",
        "FSLH" | "SLASH" => "/",
        "BSLH" | "BACKSLASH" => "\\",
        "PLUS" => "+",
        "MINUS" => "-",
        "EQUAL" => "=",
        "KP_MULTIPLY" => "×",
        "PERCENT" | "PRCNT" => "%",
        "AMPS" | "AMPERSAND" => "&",
        "ASTRK" | "ASTERISK" => "*",
        "DOLLAR" | "DLLR" => "$",
        "CARET" => "^",
        "EXCL" | "EXCLAMATION" => "!",
        "AT" | "AT_SIGN" => "@",
        "HASH" | "POUND" => "#",
        "PIPE" => "|",
        "GRAVE" => "`",
        "TILDE" => "~",
        "UNDER" | "UNDERSCORE" => "_",
        "LPAR" | "LEFT_PARENTHESIS" => "(",
        "RPAR" | "RIGHT_PARENTHESIS" => ")",
        "LBKT" | "LEFT_BRACKET" => "[",
        "RBKT" | "RIGHT_BRACKET" => "]",
        "LBRC" | "LEFT_BRACE" => "{",
        "RBRC" | "RIGHT_BRACE" => "}",
        "PAGE_UP" | "PG_UP" => "PgUp",
        "PAGE_DOWN" | "PG_DN" => "PgDn",
        "HOME" => "Home",
        "END" => "End",
        "INS" | "INSERT" => "Ins",
        "DEL" | "DELETE" => "Del",
        "SCROLLLOCK" | "SLCK" => "ScrLk",
        "PAUSE_BREAK" => "Pause",
        "C_RESET" => "Reset",
        _ => {
            if let Some(digit) = code
                .strip_prefix("NUMBER_")
                .or_else(|| code.strip_prefix('N'))
                && digit.len() == 1
                && digit.as_bytes()[0].is_ascii_digit()
            {
                return Legend::text(digit);
            }
            return Legend::text(code);
        }
    };
    Legend::text(text)
}

#[allow(clippy::too_many_arguments)]
pub fn paint(
    legend: &Legend,
    center: Point<Pixels>,
    width: f32,
    font_size: f32,
    icon_size: f32,
    color: Hsla,
    weight: FontWeight,
    window: &mut Window,
    cx: &mut App,
) {
    if legend.empty() {
        return;
    }
    if let Some(path) = legend.icon {
        let side = icon_size.min(width);
        if window
            .paint_svg(
                Bounds::new(
                    point(center.x - px(side / 2.), center.y - px(side / 2.)),
                    size(px(side), px(side)),
                ),
                path.into(),
                Default::default(),
                color,
                cx,
            )
            .is_ok()
        {
            return;
        }
    }
    let text = if legend.icon.is_some() {
        &legend.name
    } else {
        &legend.text
    };
    let shape = |text: String, font_size: f32| {
        let text: SharedString = text.into();
        let run = TextRun {
            len: text.len(),
            font: Font {
                weight,
                ..font("Inter Variable")
            },
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        window
            .text_system()
            .shape_line(text, px(font_size), &[run], None)
    };
    let mut font_size = font_size;
    let mut line = shape(text.clone(), font_size);
    if f32::from(line.width) > width {
        font_size = (font_size * width / f32::from(line.width)).max(8.);
        line = shape(text.clone(), font_size);
    }
    if f32::from(line.width) > width {
        let mut chars: Vec<_> = text.chars().collect();
        while chars.len() > 1 && f32::from(line.width) > width {
            chars.pop();
            line = shape(format!("{}…", chars.iter().collect::<String>()), font_size);
        }
    }
    let _ = line.paint(
        point(center.x - line.width / 2., center.y - px(font_size * 0.65)),
        px(font_size * 1.4),
        window,
        cx,
    );
}
