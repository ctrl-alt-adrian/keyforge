use crate::{Error, Result, layout::Keyboard};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayerId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyPosition(pub usize);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyCode(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Behavior {
    KeyPress,
    ModTap,
    LayerTap,
    MomentaryLayer,
    Transparent,
    None,
    Custom,
}
impl Behavior {
    pub const ALL: [Self; 7] = [
        Self::KeyPress,
        Self::ModTap,
        Self::LayerTap,
        Self::MomentaryLayer,
        Self::Transparent,
        Self::None,
        Self::Custom,
    ];
    pub fn description(self) -> &'static str {
        match self {
            Self::KeyPress => "Send a key while held.",
            Self::ModTap => "Hold for a modifier; tap for a key.",
            Self::LayerTap => "Hold for a layer; tap for a key.",
            Self::MomentaryLayer => "Activate a layer while held.",
            Self::Transparent => "Fall through to the next active layer.",
            Self::None => "Ignore this position.",
            Self::Custom => "A ZMK behavior or macro reference, preserved as written.",
        }
    }
    pub fn parameter_labels(self) -> &'static [&'static str] {
        match self {
            Self::KeyPress => &["Key code"],
            Self::ModTap => &["Hold modifier", "Tap key"],
            Self::LayerTap => &["Hold layer", "Tap key"],
            Self::MomentaryLayer => &["Layer"],
            Self::Transparent | Self::None => &[],
            Self::Custom => &["ZMK binding"],
        }
    }
}
impl fmt::Display for Behavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::KeyPress => "Key Press",
            Self::ModTap => "Mod-Tap",
            Self::LayerTap => "Layer-Tap",
            Self::MomentaryLayer => "Momentary Layer",
            Self::Transparent => "Transparent",
            Self::None => "None",
            Self::Custom => "Custom behavior",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Binding {
    KeyPress(KeyCode),
    ModTap { hold: KeyCode, tap: KeyCode },
    LayerTap { layer: LayerId, tap: KeyCode },
    MomentaryLayer(LayerId),
    Transparent,
    None,
    Custom(String),
}
impl Binding {
    pub fn behavior(&self) -> Behavior {
        match self {
            Self::KeyPress(_) => Behavior::KeyPress,
            Self::ModTap { .. } => Behavior::ModTap,
            Self::LayerTap { .. } => Behavior::LayerTap,
            Self::MomentaryLayer(_) => Behavior::MomentaryLayer,
            Self::Transparent => Behavior::Transparent,
            Self::None => Behavior::None,
            Self::Custom(_) => Behavior::Custom,
        }
    }
    pub fn zmk(&self) -> String {
        match self {
            Self::KeyPress(code) => format!("&kp {}", code.0),
            Self::ModTap { hold, tap } => format!("&mt {} {}", hold.0, tap.0),
            Self::LayerTap { layer, tap } => format!("&lt {} {}", layer.0, tap.0),
            Self::MomentaryLayer(layer) => format!("&mo {}", layer.0),
            Self::Transparent => "&trans".into(),
            Self::None => "&none".into(),
            Self::Custom(source) => source.clone(),
        }
    }
    pub fn label(&self) -> String {
        match self {
            Self::KeyPress(code)
            | Self::ModTap { tap: code, .. }
            | Self::LayerTap { tap: code, .. } => code.0.clone(),
            Self::MomentaryLayer(layer) => format!("L{}", layer.0),
            Self::Transparent => "▽".into(),
            Self::None => "—".into(),
            Self::Custom(source) => source
                .split_whitespace()
                .last()
                .unwrap_or(source)
                .trim_start_matches('&')
                .into(),
        }
    }
    pub fn secondary_label(&self) -> String {
        match self {
            Self::ModTap { hold, .. } => hold.0.clone(),
            Self::LayerTap { layer, .. } => format!("L{}", layer.0),
            Self::MomentaryLayer(_) => "&mo".into(),
            Self::Custom(source) => source.split_whitespace().next().unwrap_or("").into(),
            _ => self.zmk().split_whitespace().next().unwrap_or("").into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingDraft {
    pub behavior: Behavior,
    pub parameters: Vec<String>,
    pub reference: Option<String>,
}
impl From<&Binding> for BindingDraft {
    fn from(binding: &Binding) -> Self {
        let parameters = match binding {
            Binding::KeyPress(k) => [k.0.clone(), String::new()],
            Binding::ModTap { hold, tap } => [hold.0.clone(), tap.0.clone()],
            Binding::LayerTap { layer, tap } => [layer.0.to_string(), tap.0.clone()],
            Binding::MomentaryLayer(layer) => [layer.0.to_string(), String::new()],
            Binding::Transparent | Binding::None => Default::default(),
            Binding::Custom(value) => [value.clone(), String::new()],
        };
        Self {
            behavior: binding.behavior(),
            parameters: parameters.to_vec(),
            reference: None,
        }
    }
}
impl BindingDraft {
    pub fn source(&self) -> String {
        if let Some(reference) = &self.reference {
            return std::iter::once(reference.clone())
                .chain(self.parameters.clone())
                .collect::<Vec<_>>()
                .join(" ");
        }
        if self.behavior == Behavior::Custom {
            self.parameters[0].clone()
        } else {
            self.preview()
        }
    }
    pub fn change_behavior(&mut self, behavior: Behavior) {
        let tap = self
            .binding()
            .ok()
            .map(|b| b.label())
            .unwrap_or_else(|| "F".into());
        self.reference = None;
        self.behavior = behavior;
        self.parameters = match behavior {
            Behavior::KeyPress => [tap, String::new()],
            Behavior::ModTap => ["LSHFT".into(), tap],
            Behavior::LayerTap => ["1".into(), tap],
            Behavior::MomentaryLayer => ["1".into(), String::new()],
            Behavior::Transparent | Behavior::None => Default::default(),
            Behavior::Custom => ["&kp F".into(), String::new()],
        }
        .to_vec();
    }
    pub fn binding(&self) -> Result<Binding> {
        if self.reference.is_some() {
            return crate::zmk::binding(&self.source());
        }
        let first = self.parameters[0].trim().to_uppercase();
        let second = self.parameters[1].trim().to_uppercase();
        let layer = || {
            first
                .parse::<usize>()
                .map(LayerId)
                .map_err(|_| Error::InvalidDocument("Layer must be a nonnegative integer".into()))
        };
        Ok(match self.behavior {
            Behavior::KeyPress => Binding::KeyPress(KeyCode(first)),
            Behavior::ModTap => Binding::ModTap {
                hold: KeyCode(first),
                tap: KeyCode(second),
            },
            Behavior::LayerTap => Binding::LayerTap {
                layer: layer()?,
                tap: KeyCode(second),
            },
            Behavior::MomentaryLayer => Binding::MomentaryLayer(layer()?),
            Behavior::Transparent => Binding::Transparent,
            Behavior::None => Binding::None,
            Behavior::Custom => crate::zmk::binding(&self.parameters[0])?,
        })
    }
    pub fn preview(&self) -> String {
        self.binding()
            .map(|b| b.zmk())
            .unwrap_or_else(|e| e.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub bindings: Vec<Binding>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keymap {
    pub layers: Vec<Layer>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repository {
    pub owner: String,
    pub name: String,
    pub branch: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Configuration {
    pub board: String,
    pub shields: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub schema_version: u32,
    pub name: String,
    pub repository: Repository,
    pub configuration: Configuration,
    pub keyboard: Keyboard,
    pub keymap: Keymap,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zmk: Option<crate::zmk::Document>,
}
impl Project {
    pub fn check_structure(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(Error::InvalidDocument(
                "Unsupported prototype schema".into(),
            ));
        }
        if self.keymap.layers.is_empty() || self.keyboard.keys.is_empty() {
            return Err(Error::InvalidDocument("Missing layers or geometry".into()));
        }
        self.keyboard.check()?;
        if let Some(source) = &self.zmk
            && source.layers()? != self.keymap.layers
        {
            return Err(Error::InvalidDocument(
                "Cached keymap data does not match its ZMK source".into(),
            ));
        }
        if self
            .keymap
            .layers
            .iter()
            .any(|l| l.bindings.len() != self.keyboard.keys.len())
        {
            return Err(Error::InvalidDocument(
                "Layer and geometry key counts differ".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Warning,
    Error,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub layer: LayerId,
    pub key: KeyPosition,
}

pub const MODIFIERS: &[&str] = &[
    "LSHFT", "RSHFT", "LCTRL", "RCTRL", "LALT", "RALT", "LGUI", "RGUI",
];
pub const SPECIAL_KEY_CODES: &[&str] = &[
    "TAB", "BSPC", "SPACE", "RET", "ESC", "SEMI", "SQT", "COMMA", "DOT", "FSLH", "LEFT", "DOWN",
    "UP", "RIGHT", "MINUS", "EQUAL", "LBKT", "RBKT", "BSLH", "GRAVE", "DEL", "HOME", "END",
    "PG_UP", "PG_DN",
];

/// Keycode names and aliases from the bundled, revision-pinned ZMK catalog.
pub fn supported_key_codes() -> Vec<String> {
    let mut codes: Vec<String> = ('A'..='Z')
        .map(|c| c.to_string())
        .chain((0..=9).map(|n| format!("N{n}")))
        .chain((1..=24).map(|n| format!("F{n}")))
        .chain(
            MODIFIERS
                .iter()
                .chain(SPECIAL_KEY_CODES)
                .map(|s| s.to_string()),
        )
        .chain(
            include_str!("../../assets/zmk/keycodes.txt")
                .lines()
                .map(str::to_string),
        )
        .chain(
            include_str!("../../assets/zmk/keycode-labels.tsv")
                .lines()
                .filter_map(|line| line.split_once('\t').map(|(code, _)| code.to_string())),
        )
        .collect();
    codes.sort();
    codes.dedup();
    codes
}
pub fn valid_key_code(code: &str) -> bool {
    if code.parse::<u32>().is_ok()
        || code
            .strip_prefix("0x")
            .or_else(|| code.strip_prefix("0X"))
            .is_some_and(|value| u32::from_str_radix(value, 16).is_ok())
        || ((code.starts_with("ZMK_HID_USAGE(") || code.starts_with('('))
            && crate::bindings::split(&format!("&kp {code}"))
                .is_ok_and(|(_, args)| args.len() == 1))
    {
        return true;
    }
    if include_str!("../../assets/zmk/keycode-labels.tsv")
        .lines()
        .any(|line| line.split_once('\t').is_some_and(|(name, _)| name == code))
    {
        return true;
    }
    if include_str!("../../assets/zmk/keycodes.txt")
        .lines()
        .any(|name| name == code)
    {
        return true;
    }
    if let Some((modifier, rest)) = code.split_once('(')
        && ["LC", "LS", "LA", "LG", "RC", "RS", "RA", "RG"].contains(&modifier)
        && let Some(inner) = rest.strip_suffix(')')
    {
        return valid_key_code(inner);
    }

    (code.len() == 1 && code.as_bytes()[0].is_ascii_uppercase())
        || (code.starts_with('N') && code.len() == 2 && code.as_bytes()[1].is_ascii_digit())
        || code
            .strip_prefix('F')
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=24).contains(&n))
        || MODIFIERS.contains(&code)
        || SPECIAL_KEY_CODES.contains(&code)
}
pub fn validate_binding(
    binding: &Binding,
    layer: LayerId,
    key: KeyPosition,
    layer_count: usize,
) -> Vec<Diagnostic> {
    let mut issues = Vec::new();
    let mut error = |message: String| {
        issues.push(Diagnostic {
            severity: Severity::Error,
            message,
            layer,
            key,
        })
    };
    match binding {
        Binding::KeyPress(k) => {
            if !valid_key_code(&k.0) {
                error(format!("Unknown key code: {}", k.0));
            }
        }
        Binding::ModTap { hold, tap } => {
            if !MODIFIERS.contains(&hold.0.as_str()) {
                error("Mod-Tap hold must be a modifier".into());
            }
            if !valid_key_code(&tap.0) {
                error(format!("Unknown tap key: {}", tap.0));
            }
        }
        Binding::LayerTap { layer: target, tap } => {
            if target.0 >= layer_count {
                error("Target layer does not exist".into());
            }
            if !valid_key_code(&tap.0) {
                error(format!("Unknown tap key: {}", tap.0));
            }
        }
        Binding::MomentaryLayer(target) => {
            if target.0 >= layer_count {
                error("Target layer does not exist".into());
            }
        }
        Binding::Transparent if layer.0 == 0 => issues.push(Diagnostic {
            severity: Severity::Warning,
            message: "Transparent on Base has no lower layer".into(),
            layer,
            key,
        }),
        Binding::None => issues.push(Diagnostic {
            severity: Severity::Warning,
            message: "This position is disabled".into(),
            layer,
            key,
        }),
        _ => {}
    }
    issues
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingChange {
    pub layer: LayerId,
    pub key: KeyPosition,
    pub before: Binding,
    pub after: Binding,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareArtifact {
    pub name: String,
    pub bytes: u64,
    pub simulated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactSet {
    pub build: u64,
    pub configuration: String,
    pub directory: Option<std::path::PathBuf>,
    pub artifacts: Vec<FirmwareArtifact>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildStatus {
    Idle,
    Queued,
    Running,
    Successful,
    Failed,
    Cancelled,
}
impl BuildStatus {
    pub fn active(self) -> bool {
        matches!(self, Self::Queued | Self::Running)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildProblem {
    pub severity: Severity,
    pub message: String,
    pub target: Option<String>,
}
/// Classify diagnostic-bearing lines while retaining their original output text.
pub fn build_line_severity(line: &str) -> Option<Severity> {
    let lower = line.trim().to_ascii_lowercase();
    if lower.contains("warning:") || lower.starts_with("cmake warning") {
        return Some(Severity::Warning);
    }
    if [
        "error:",
        "fatal error:",
        "cmake error",
        "runtimeerror:",
        "valueerror:",
        "exception:",
        "local build error:",
        "local build setup:",
        "traceback (",
        "failed:",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
        || lower.contains("local firmware build failed")
        || lower.contains("build environment setup failed")
    {
        Some(Severity::Error)
    } else {
        None
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build {
    pub id: u64,
    pub status: BuildStatus,
    pub progress: u8,
    /// Individual log lines, retained for the current build, including failures.
    pub output: Vec<String>,
    pub artifacts: Vec<FirmwareArtifact>,
    pub problems: Vec<BuildProblem>,
    pub current_target: Option<String>,
}
impl Default for Build {
    fn default() -> Self {
        Self {
            current_target: None,
            id: 0,
            status: BuildStatus::Idle,
            progress: 0,
            output: vec![
                "SIMULATION · Ready. No firmware compiler or GitHub workflow will run.".into(),
            ],
            artifacts: Vec::new(),
            problems: Vec::new(),
        }
    }
}
