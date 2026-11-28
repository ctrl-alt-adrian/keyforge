//! Editable ZMK behavior metadata. Bindings remain source expressions, so module
//! behaviors and parameters never need a new serialized Binding variant.
use crate::{
    Binding, BindingDraft, Error, Result,
    zmk::{self, Document, Node},
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    pub reference: String,
    pub arguments: Vec<String>,
}
impl Form {
    pub fn source(&self) -> String {
        draft(&self.reference, &self.arguments).source()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterKind {
    KeyCode,
    Layer,
    Choice(Vec<String>),
    Expression,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub label: String,
    pub kind: ParameterKind,
    pub default: String,
}
impl Parameter {
    fn new(label: impl Into<String>, kind: ParameterKind, default: &str) -> Self {
        Self {
            label: label.into(),
            kind,
            default: default.into(),
        }
    }
    fn code(label: &str) -> Self {
        Self::new(label, ParameterKind::KeyCode, "A")
    }
    fn layer(label: &str) -> Self {
        Self::new(label, ParameterKind::Layer, "0")
    }
    fn expression(label: impl Into<String>) -> Self {
        Self::new(label, ParameterKind::Expression, "0")
    }
    fn choices(label: &str, values: &[&str]) -> Self {
        Self::new(
            label,
            ParameterKind::Choice(values.iter().map(|s| (*s).into()).collect()),
            values[0],
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorSpec {
    pub reference: String,
    pub name: String,
    pub description: String,
    pub parameters: Vec<Parameter>,
    pub header: Option<String>,
}
impl BehaviorSpec {
    pub fn presentation_order(&self, arguments: &[String]) -> Vec<usize> {
        let fields = self.fields(arguments);
        let mut indices = (0..fields.len()).collect::<Vec<_>>();
        indices.sort_by_key(|index| {
            if fields[*index].label.to_lowercase().starts_with("tap") {
                0
            } else {
                1
            }
        });
        indices
    }
    fn new(
        reference: &str,
        name: &str,
        description: &str,
        parameters: Vec<Parameter>,
        header: Option<&str>,
    ) -> Self {
        Self {
            reference: reference.into(),
            name: name.into(),
            description: description.into(),
            parameters,
            header: header.map(str::to_string),
        }
    }
    pub fn title(&self) -> String {
        format!("{} · {}", self.reference, self.name)
    }
    /// Some command macros already expand to both binding cells. Keep the
    /// optional value field only for actions which take an explicit value.
    pub fn fields(&self, arguments: &[String]) -> Vec<Parameter> {
        let mut fields = self.parameters.clone();
        let command = arguments.first().map(String::as_str).unwrap_or("");
        let collapsed = match self.reference.as_str() {
            "&bt" => ["BT_CLR", "BT_NXT", "BT_PRV", "BT_CLR_ALL"].contains(&command),
            "&bl" => {
                ["BL_ON", "BL_OFF", "BL_TOG", "BL_INC", "BL_DEC", "BL_CYCLE"].contains(&command)
            }
            "&rgb_ug" => {
                [
                    "RGB_TOG", "RGB_ON", "RGB_OFF", "RGB_HUI", "RGB_HUD", "RGB_SAI", "RGB_SAD",
                    "RGB_BRI", "RGB_BRD", "RGB_SPI", "RGB_SPD", "RGB_EFF", "RGB_EFR",
                ]
                .contains(&command)
                    || command.starts_with("RGB_COLOR_HSB(")
                    || command.starts_with("RGB_COLOR_HSV(")
            }
            _ => false,
        };
        if collapsed {
            fields.truncate(1);
        }
        fields
    }
}

/// All upstream key-position behaviors at the revision recorded in assets/zmk.
/// Sensor and macro-control behaviors belong to their own editing contexts.
pub fn catalog(document: Option<&Document>) -> Vec<BehaviorSpec> {
    let code = || vec![Parameter::code("Key code")];
    let layer = || vec![Parameter::layer("Layer")];
    let mut specs = vec![
        BehaviorSpec::new(
            "&kp",
            "Key Press",
            "Send a keycode while held, with optional modifiers.",
            code(),
            Some("keys.h"),
        ),
        BehaviorSpec::new(
            "&mt",
            "Mod-Tap",
            "Hold for a modifier or modified key; tap for another key.",
            vec![
                Parameter::new("Hold key / modifiers", ParameterKind::KeyCode, "LCTRL"),
                Parameter::code("Tap key"),
            ],
            Some("keys.h"),
        ),
        BehaviorSpec::new(
            "&lt",
            "Layer-Tap",
            "Hold for a layer; tap for a key.",
            vec![Parameter::layer("Hold layer"), Parameter::code("Tap key")],
            Some("keys.h"),
        ),
        BehaviorSpec::new(
            "&mo",
            "Momentary Layer",
            "Activate a layer while held.",
            layer(),
            None,
        ),
        BehaviorSpec::new(
            "&to",
            "To Layer",
            "Activate a layer and disable the other non-default layers.",
            layer(),
            None,
        ),
        BehaviorSpec::new(
            "&tog",
            "Toggle Layer",
            "Toggle a layer on or off.",
            layer(),
            None,
        ),
        BehaviorSpec::new(
            "&sl",
            "Sticky Layer",
            "Activate a layer for the next keypress.",
            layer(),
            None,
        ),
        BehaviorSpec::new(
            "&kt",
            "Key Toggle",
            "Toggle a key between pressed and released.",
            code(),
            Some("keys.h"),
        ),
        BehaviorSpec::new(
            "&sk",
            "Sticky Key",
            "Hold a key or modifier until the next keypress.",
            code(),
            Some("keys.h"),
        ),
        BehaviorSpec::new(
            "&gresc",
            "Grave / Escape",
            "Send Escape, or Grave when Shift or GUI is held.",
            vec![],
            None,
        ),
        BehaviorSpec::new(
            "&caps_word",
            "Caps Word",
            "Capitalize the next word.",
            vec![],
            None,
        ),
        BehaviorSpec::new(
            "&key_repeat",
            "Key Repeat",
            "Repeat the last keycode.",
            vec![],
            None,
        ),
        BehaviorSpec::new(
            "&trans",
            "Transparent",
            "Fall through to the next active layer.",
            vec![],
            None,
        ),
        BehaviorSpec::new("&none", "None", "Ignore this key position.", vec![], None),
        BehaviorSpec::new(
            "&bt",
            "Bluetooth",
            "Manage Bluetooth profiles and pairing.",
            vec![
                Parameter::choices(
                    "Action",
                    &[
                        "BT_SEL",
                        "BT_NXT",
                        "BT_PRV",
                        "BT_CLR",
                        "BT_CLR_ALL",
                        "BT_DISC",
                    ],
                ),
                Parameter::expression("Profile (zero-based)"),
            ],
            Some("bt.h"),
        ),
        BehaviorSpec::new(
            "&out",
            "Output Selection",
            "Select USB, Bluetooth, no output, or toggle outputs.",
            vec![Parameter::choices(
                "Output",
                &["OUT_TOG", "OUT_USB", "OUT_BLE", "OUT_NONE"],
            )],
            Some("outputs.h"),
        ),
        BehaviorSpec::new(
            "&rgb_ug",
            "RGB Underglow",
            "Control underglow; requires RGB support in the firmware.",
            vec![
                Parameter::choices(
                    "Action",
                    &[
                        "RGB_TOG",
                        "RGB_ON",
                        "RGB_OFF",
                        "RGB_HUI",
                        "RGB_HUD",
                        "RGB_SAI",
                        "RGB_SAD",
                        "RGB_BRI",
                        "RGB_BRD",
                        "RGB_SPI",
                        "RGB_SPD",
                        "RGB_EFF",
                        "RGB_EFR",
                        "RGB_EFS_CMD",
                        "RGB_COLOR_HSB_CMD",
                        "RGB_COLOR_HSB(0, 100, 100)",
                    ],
                ),
                Parameter::expression("Effect index / packed HSB color"),
            ],
            Some("rgb.h"),
        ),
        BehaviorSpec::new(
            "&bl",
            "Backlight",
            "Control backlighting; requires backlight support in the firmware.",
            vec![
                Parameter::choices(
                    "Action",
                    &[
                        "BL_TOG", "BL_ON", "BL_OFF", "BL_INC", "BL_DEC", "BL_CYCLE", "BL_SET",
                    ],
                ),
                Parameter::expression("Brightness (0–100)"),
            ],
            Some("backlight.h"),
        ),
        BehaviorSpec::new(
            "&ext_power",
            "External Power",
            "Control external power on supported hardware.",
            vec![Parameter::choices("Action", &["EP_TOG", "EP_ON", "EP_OFF"])],
            Some("ext_power.h"),
        ),
        BehaviorSpec::new(
            "&mkp",
            "Mouse Button Press",
            "Press a mouse button; requires pointing support in the firmware.",
            vec![Parameter::choices(
                "Mouse button",
                &["LCLK", "RCLK", "MCLK", "MB4", "MB5", "(MB1 | MB2)"],
            )],
            Some("pointing.h"),
        ),
        BehaviorSpec::new(
            "&mmv",
            "Mouse Move",
            "Move the pointer. MOVE(x, y) sets custom horizontal and vertical speeds.",
            vec![Parameter::choices(
                "Direction / velocity",
                &[
                    "MOVE_UP",
                    "MOVE_DOWN",
                    "MOVE_LEFT",
                    "MOVE_RIGHT",
                    "MOVE(600, 600)",
                ],
            )],
            Some("pointing.h"),
        ),
        BehaviorSpec::new(
            "&msc",
            "Mouse Scroll",
            "Scroll horizontally or vertically; requires pointing support.",
            vec![Parameter::choices(
                "Direction / velocity",
                &[
                    "SCRL_UP",
                    "SCRL_DOWN",
                    "SCRL_LEFT",
                    "SCRL_RIGHT",
                    "MOVE(0, 10)",
                ],
            )],
            Some("pointing.h"),
        ),
        BehaviorSpec::new(
            "&sys_reset",
            "Reset",
            "Restart the keyboard firmware.",
            vec![],
            None,
        ),
        BehaviorSpec::new(
            "&bootloader",
            "Bootloader",
            "Restart into the keyboard's bootloader.",
            vec![],
            None,
        ),
        BehaviorSpec::new(
            "&soft_off",
            "Soft Off",
            "Turn the keyboard off on supported hardware.",
            vec![],
            None,
        ),
        BehaviorSpec::new(
            "&studio_unlock",
            "Studio Unlock",
            "Unlock changes from ZMK Studio.",
            vec![],
            None,
        ),
    ];
    if let Some(doc) = document
        && let Ok(tree) = doc.tree()
    {
        collect(&tree, &doc.source, &mut specs);
        // Resolve parameter kinds through child behaviors, including custom
        // wrappers. Each pass advances at least one level of an acyclic chain.
        for _ in 0..specs.len() {
            let before = specs.clone();
            resolve(&tree, &doc.source, &before, &mut specs);
            if specs == before {
                break;
            }
        }
    }
    specs.sort_by(|a, b| a.reference.cmp(&b.reference));
    specs
}

fn collect(node: &Node, source: &str, specs: &mut Vec<BehaviorSpec>) {
    if let Some(label) = &node.label
        && let Some(cells) = node
            .value(source, "#binding-cells")
            .and_then(|v| zmk::cells(v).parse::<usize>().ok())
        && cells <= 32
        && node
            .value(source, "compatible")
            .is_some_and(|v| v.contains("zmk,behavior-"))
    {
        let reference = format!("&{label}");
        let name = node
            .value(source, "display-name")
            .map(zmk::unquote)
            .or_else(|| node.value(source, "label").map(zmk::unquote))
            .unwrap_or_else(|| label.clone());
        let parameters = (0..cells)
            .map(|i| Parameter::expression(format!("Parameter {}", i + 1)))
            .collect();
        let spec = BehaviorSpec::new(
            &reference,
            &name,
            "Behavior defined in this configuration.",
            parameters,
            None,
        );
        if let Some(existing) = specs.iter_mut().find(|s| s.reference == reference) {
            *existing = spec;
        } else {
            specs.push(spec);
        }
    }
    for child in &node.children {
        collect(child, source, specs);
    }
}

fn resolve(node: &Node, source: &str, known: &[BehaviorSpec], specs: &mut [BehaviorSpec]) {
    if let Some(label) = &node.label
        && let Some(spec) = specs
            .iter_mut()
            .find(|s| s.reference == format!("&{label}"))
    {
        let kind = node
            .value(source, "compatible")
            .map(zmk::unquote)
            .unwrap_or_default();
        let children = node
            .value(source, "bindings")
            .and_then(|v| zmk::binding_spans(v).ok().map(|spans| (v, spans)));
        if let Some((value, spans)) = children {
            let refs: Vec<_> = spans
                .iter()
                .filter_map(|span| split(&value[span.clone()]).ok())
                .collect();
            if kind == "zmk,behavior-hold-tap" && spec.parameters.len() == 2 && refs.len() == 2 {
                for (i, prefix) in ["Hold", "Tap"].iter().enumerate() {
                    if let Some(child) = known.iter().find(|s| s.reference == refs[i].0)
                        && let Some(parameter) = child.parameters.first()
                        && refs[i].1.is_empty()
                    {
                        spec.parameters[i] = parameter.clone();
                        spec.parameters[i].label =
                            format!("{prefix}: {}", parameter.label.to_lowercase());
                    }
                }
            } else if kind == "zmk,behavior-sticky-key"
                && spec.parameters.len() == 1
                && refs.len() == 1
                && let Some(child) = known.iter().find(|s| s.reference == refs[0].0)
                && let Some(parameter) = child.parameters.first()
            {
                spec.parameters[0] = parameter.clone();
            }
        }
        if ["zmk,behavior-key-press", "zmk,behavior-key-toggle"].contains(&kind.as_str())
            && spec.parameters.len() == 1
        {
            spec.parameters[0] = Parameter::code("Key code");
        }
        if [
            "zmk,behavior-momentary-layer",
            "zmk,behavior-to-layer",
            "zmk,behavior-toggle-layer",
        ]
        .contains(&kind.as_str())
            && spec.parameters.len() == 1
        {
            spec.parameters[0] = Parameter::layer("Layer");
        }
    }
    for child in &node.children {
        resolve(child, source, known, specs);
    }
}

/// Split top-level arguments without splitting function calls or parenthesized
/// C expressions. No preprocessing or evaluation of user source is performed.
pub fn split(source: &str) -> Result<(String, Vec<String>)> {
    let source = source.trim();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    for (i, c) in source.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| invalid("Unbalanced parameter expression"))?
            }
            '{' | '}' | ';' | '"' | '#' | '[' | ']' => {
                return Err(invalid("Enter one binding without Devicetree delimiters"));
            }
            '<' | '>' | ',' if depth == 0 => {
                return Err(invalid(
                    "Parameter expressions containing operators or commas must be parenthesized",
                ));
            }
            c if c.is_whitespace() && depth == 0 => {
                if start < i {
                    parts.push(source[start..i].to_string());
                }
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(invalid("Unbalanced parameter expression"));
    }
    if start < source.len() {
        parts.push(source[start..].to_string());
    }
    let reference = parts
        .first()
        .ok_or_else(|| invalid("Choose a behavior"))?
        .clone();
    if !reference.starts_with('&')
        || !zmk::identifier(&reference[1..])
        || parts.iter().skip(1).any(|p| p.starts_with('&'))
        || source.contains("//")
        || source.contains("/*")
    {
        return Err(invalid(
            "Enter a single &behavior followed by its parameters",
        ));
    }
    Ok((reference, parts.into_iter().skip(1).collect()))
}
fn invalid(message: &str) -> Error {
    Error::InvalidDocument(message.into())
}
/// Tolerant counterpart used only while editing: an unfinished function call
/// remains in its own field until validation can accept the completed draft.
pub fn editing_parts(source: &str) -> (String, Vec<String>) {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    for (i, c) in source.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            c if c.is_whitespace() && depth == 0 => {
                if start < i {
                    parts.push(source[start..i].to_string());
                }
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    if start < source.len() {
        parts.push(source[start..].to_string());
    }
    let reference = if parts.is_empty() {
        String::new()
    } else {
        parts.remove(0)
    };
    (reference, parts)
}
pub fn draft(reference: &str, arguments: &[String]) -> BindingDraft {
    BindingDraft {
        behavior: crate::Behavior::Custom,
        parameters: arguments.to_vec(),
        reference: Some(reference.into()),
    }
}

pub const MODIFIER_FUNCTIONS: [(&str, &str); 8] = [
    ("LC", "LCtrl"),
    ("LS", "LShift"),
    ("LA", "LAlt"),
    ("LG", "LGui"),
    ("RC", "RCtrl"),
    ("RS", "RShift"),
    ("RA", "RAlt"),
    ("RG", "RGui"),
];
pub fn modifiers(value: &str) -> (String, Vec<String>) {
    let mut code = value.trim();
    let mut modifiers = Vec::new();
    while let Some((name, rest)) = code.split_once('(') {
        if !MODIFIER_FUNCTIONS
            .iter()
            .any(|(function, _)| *function == name)
        {
            break;
        }
        let Some(inner) = rest.strip_suffix(')') else {
            break;
        };
        if split(&format!("&kp {inner}")).is_err() {
            break;
        }
        modifiers.push(name.to_string());
        code = inner.trim();
    }
    (code.to_string(), modifiers)
}
pub fn with_modifiers(code: &str, modifiers: &[String]) -> String {
    modifiers
        .iter()
        .rev()
        .fold(code.to_string(), |value, modifier| {
            format!("{modifier}({value})")
        })
}

pub fn primary_modifier(code: &str) -> Option<&'static str> {
    match code {
        "LCTRL" | "LEFT_CONTROL" | "LEFT_CTRL" => Some("LC"),
        "LSHFT" | "LEFT_SHIFT" => Some("LS"),
        "LALT" | "LEFT_ALT" => Some("LA"),
        "LGUI" | "LEFT_GUI" | "LEFT_WIN" | "LEFT_COMMAND" => Some("LG"),
        "RCTRL" | "RIGHT_CONTROL" | "RIGHT_CTRL" => Some("RC"),
        "RSHFT" | "RIGHT_SHIFT" => Some("RS"),
        "RALT" | "RIGHT_ALT" | "ALTGR" => Some("RA"),
        "RGUI" | "RIGHT_GUI" | "RIGHT_WIN" | "RIGHT_COMMAND" => Some("RG"),
        _ => None,
    }
}
pub fn intrinsic_modifiers(code: &str) -> Vec<String> {
    let primary = primary_modifier(code);
    if let Some(primary) = primary {
        return vec![primary.into()];
    }
    include_str!("../../assets/zmk/intrinsic-modifiers.tsv")
        .lines()
        .find_map(|line| {
            let (name, modifiers) = line.split_once('\t')?;
            (name == code).then(|| modifiers.split(',').map(str::to_string).collect())
        })
        .unwrap_or_default()
}
pub fn summary(value: &str, kind: &ParameterKind, layers: &[crate::Layer]) -> String {
    if *kind == ParameterKind::Layer {
        return value
            .parse::<usize>()
            .ok()
            .and_then(|i| layers.get(i).map(|layer| format!("{i} — {}", layer.name)))
            .unwrap_or_else(|| value.into());
    }
    if *kind != ParameterKind::KeyCode {
        return value.into();
    }
    let (code, modifiers) = modifiers(value);
    let primary_name = primary_modifier(&code).and_then(|modifier| {
        MODIFIER_FUNCTIONS
            .iter()
            .find(|(name, _)| *name == modifier)
            .map(|(_, label)| {
                label
                    .replacen('L', "Left ", 1)
                    .replacen('R', "Right ", 1)
                    .replace("Gui", "GUI")
            })
    });
    let name = include_str!("../../assets/zmk/keycode-labels.tsv")
        .lines()
        .find_map(|line| {
            let (name, description) = line.split_once('\t')?;
            (name == code).then(|| {
                description
                    .strip_prefix("Keyboard ")
                    .unwrap_or(description)
                    .to_string()
            })
        })
        .unwrap_or_else(|| code.clone());
    let name = if let Some(primary) = primary_name {
        primary
    } else if code.len() == 1 || code.starts_with('F') && code[1..].parse::<u8>().is_ok() {
        code.clone()
    } else {
        name
    };
    let mut parts = modifiers
        .iter()
        .filter_map(|m| {
            MODIFIER_FUNCTIONS
                .iter()
                .find(|(name, _)| name == m)
                .map(|(_, label)| (*label).to_string())
        })
        .collect::<Vec<_>>();
    parts.push(name);
    parts.join(" + ")
}

pub fn macro_catalog(document: Option<&Document>) -> Vec<BehaviorSpec> {
    let mut specs = catalog(document);
    for (reference, label, parameters) in [
        ("&macro_tap", "Tap mode", vec![]),
        ("&macro_press", "Press mode", vec![]),
        ("&macro_release", "Release mode", vec![]),
        ("&macro_pause_for_release", "Pause until release", vec![]),
        (
            "&macro_wait_time",
            "Set wait time",
            vec![Parameter::expression("Wait time (ms)")],
        ),
        (
            "&macro_tap_time",
            "Set tap time",
            vec![Parameter::expression("Tap time (ms)")],
        ),
        ("&macro_param_1to1", "Forward parameter 1 to 1", vec![]),
        ("&macro_param_1to2", "Forward parameter 1 to 2", vec![]),
        ("&macro_param_2to1", "Forward parameter 2 to 1", vec![]),
        ("&macro_param_2to2", "Forward parameter 2 to 2", vec![]),
    ] {
        specs.push(BehaviorSpec::new(
            reference,
            label,
            "Macro control operation; execution context is preserved.",
            parameters,
            None,
        ));
    }
    specs
}

pub fn sensor_catalog(document: Option<&Document>) -> Vec<BehaviorSpec> {
    let mut specs = vec![BehaviorSpec::new(
        "&inc_dec_kp",
        "Key press rotation",
        "Clockwise and counterclockwise actions, in sensor binding order.",
        vec![
            Parameter::new("Clockwise", ParameterKind::KeyCode, "C_VOL_UP"),
            Parameter::new("Counterclockwise", ParameterKind::KeyCode, "C_VOL_DN"),
        ],
        Some("keys.h"),
    )];
    fn collect(node: &Node, source: &str, specs: &mut Vec<BehaviorSpec>, keys: &[BehaviorSpec]) {
        if let Some(label) = &node.label
            && let Some(cells) = node
                .value(source, "#sensor-binding-cells")
                .and_then(|s| zmk::cells(s).parse::<usize>().ok())
            && cells <= 32
        {
            let mut parameters = (0..cells)
                .map(|i| Parameter::expression(format!("Parameter {}", i + 1)))
                .collect::<Vec<_>>();
            if let Some(value) = node.value(source, "bindings")
                && let Ok(spans) = zmk::binding_spans(value)
            {
                for (i, span) in spans.iter().take(cells).enumerate() {
                    if let Ok((reference, _)) = split(&value[span.clone()])
                        && let Some(spec) = keys.iter().find(|spec| spec.reference == reference)
                        && let Some(field) = spec.parameters.first()
                    {
                        parameters[i] = field.clone();
                        parameters[i].label = match i {
                            0 => "Clockwise".into(),
                            1 => "Counterclockwise".into(),
                            _ => format!("Parameter {}", i + 1),
                        };
                    }
                }
            }
            specs.push(BehaviorSpec::new(
                &format!("&{label}"),
                label,
                "Sensor behavior defined by this configuration.",
                parameters,
                None,
            ));
        }
        for child in &node.children {
            collect(child, source, specs, keys);
        }
    }
    if let Some(document) = document
        && let Ok(tree) = document.tree()
    {
        collect(
            &tree,
            &document.source,
            &mut specs,
            &catalog(Some(document)),
        );
    }
    specs
}

pub struct DefinitionPreview {
    pub path: String,
    pub section: crate::editor::Section,
    pub kind: String,
    pub lines: Vec<String>,
}
pub fn definition_preview(document: &Document, reference: &str) -> Option<DefinitionPreview> {
    fn find<'a>(node: &'a Node, label: &str) -> Option<&'a Node> {
        if node.label.as_deref() == Some(label) {
            return Some(node);
        }
        node.children.iter().find_map(|child| find(child, label))
    }
    let tree = document.tree().ok()?;
    let node = find(&tree, reference.strip_prefix('&')?)?;
    let kind = node
        .value(&document.source, "compatible")
        .map(zmk::unquote)?;
    let section = if kind.starts_with("zmk,behavior-macro") {
        crate::editor::Section::Macros
    } else {
        crate::editor::Section::Behaviors
    };
    let mut lines = Vec::new();
    if let Some(value) = node.value(&document.source, "bindings") {
        for (i, span) in zmk::binding_spans(value).ok()?.iter().enumerate() {
            let label = if kind == "zmk,behavior-hold-tap" {
                if i == 0 { "Hold".into() } else { "Tap".into() }
            } else if kind == "zmk,behavior-tap-dance" {
                format!("{} tap(s)", i + 1)
            } else {
                format!("Action {}", i + 1)
            };
            lines.push(format!("{label} · {}", &value[span.clone()]));
        }
    }
    for property in [
        "flavor",
        "tapping-term-ms",
        "quick-tap-ms",
        "wait-ms",
        "tap-ms",
    ] {
        if let Some(value) = node.value(&document.source, property) {
            lines.push(format!("{property} · {}", zmk::unquote(&zmk::cells(value))));
        }
    }
    Some(DefinitionPreview {
        path: node.path.clone(),
        section,
        kind,
        lines,
    })
}

pub fn keycode_choices() -> Vec<(String, String)> {
    let descriptions: BTreeMap<_, _> = include_str!("../../assets/zmk/keycode-labels.tsv")
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect();
    crate::supported_key_codes()
        .into_iter()
        .map(|code| {
            let description = descriptions
                .get(code.as_str())
                .copied()
                .unwrap_or("")
                .to_string();
            (code, description)
        })
        .collect()
}

pub fn validate(binding: &Binding, catalog: &[BehaviorSpec], layer_count: usize) -> Vec<String> {
    let Ok((reference, arguments)) = split(&binding.zmk()) else {
        return vec!["Invalid binding expression".into()];
    };
    validate_form(
        &Form {
            reference,
            arguments,
        },
        catalog,
        layer_count,
    )
}
pub fn validate_form(form: &Form, catalog: &[BehaviorSpec], layer_count: usize) -> Vec<String> {
    let Form {
        reference,
        arguments,
    } = form;
    let Some(spec) = catalog.iter().find(|s| s.reference == *reference) else {
        return vec![];
    };
    let fields = spec.fields(arguments);
    let mut errors = Vec::new();
    if fields.len() != arguments.len() {
        errors.push(format!(
            "{} needs {} parameter(s); got {}",
            spec.reference,
            fields.len(),
            arguments.len()
        ));
    }
    for (field, value) in fields.iter().zip(arguments) {
        if value.is_empty() {
            errors.push(format!("{} is required", field.label));
        }
        if field.kind == ParameterKind::Layer
            && let Ok(layer) = value.parse::<usize>()
            && layer >= layer_count
        {
            errors.push(format!(
                "{} refers to a layer that does not exist",
                field.label
            ));
        }
    }
    errors
}

/// Includes needed by the selected action and its direct keycode parameters.
pub fn headers(binding: &str) -> Vec<String> {
    let Ok((reference, args)) = split(binding) else {
        return vec![];
    };
    let mut headers = vec![];
    if let Some(spec) = catalog(None).iter().find(|s| s.reference == reference)
        && let Some(header) = &spec.header
    {
        headers.push(header.clone());
    }
    if args.iter().any(|arg| crate::valid_key_code(arg)) && !headers.iter().any(|h| h == "keys.h") {
        headers.push("keys.h".into());
    }
    headers
}
