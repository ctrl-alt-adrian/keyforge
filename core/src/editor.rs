//! Domain editing forms for the remaining ZMK sections. These values describe
//! ZMK properties, not GUI widgets; all mutations are applied by Application.
use crate::{
    Error, Project, Result,
    zmk::{self, Document, Node},
};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Combos,
    Macros,
    Behaviors,
    ConditionalLayers,
    Encoders,
}
impl Section {
    pub fn title(self) -> &'static str {
        match self {
            Self::Combos => "Combos",
            Self::Macros => "Macros",
            Self::Behaviors => "Behaviors",
            Self::ConditionalLayers => "Conditional layers",
            Self::Encoders => "Encoders",
        }
    }
    pub fn singular(self) -> &'static str {
        match self {
            Self::Combos => "combo",
            Self::Macros => "macro",
            Self::Behaviors => "behavior",
            Self::ConditionalLayers => "rule",
            Self::Encoders => "encoder binding",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Combos => "Select a combo, then click its key positions on the keyboard.",
            Self::Macros => "Create reusable behavior sequences and bind them to keys.",
            Self::Behaviors => "Configure custom ZMK behavior definitions.",
            Self::ConditionalLayers => "Activate a layer when all prerequisite layers are active.",
            Self::Encoders => "Edit the sensor bindings defined by this keymap, for each layer.",
        }
    }
    fn group(self) -> (&'static str, Option<&'static str>) {
        match self {
            Self::Combos => ("combos", Some("zmk,combos")),
            Self::Macros => ("macros", None),
            Self::Behaviors => ("behaviors", None),
            Self::ConditionalLayers => ("conditional_layers", Some("zmk,conditional-layers")),
            Self::Encoders => ("", None),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub detail: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Text,
    Number,
    Binding,
    Bindings,
    Positions,
    Layers,
    Layer,
    Choice(Vec<String>),
}
#[derive(Debug, Clone)]
pub struct Field {
    pub id: String,
    pub label: String,
    pub value: String,
    pub initial: String,
    pub kind: Kind,
}
impl Field {
    fn new(id: &str, label: &str, value: impl Into<String>, kind: Kind) -> Self {
        let value = value.into();
        Self {
            id: id.into(),
            label: label.into(),
            initial: value.clone(),
            value,
            kind,
        }
    }
    pub fn changed(&self) -> bool {
        self.value != self.initial
    }
}
#[derive(Debug, Clone)]
pub struct Draft {
    pub section: Section,
    pub node: Option<String>,
    pub fields: Vec<Field>,
    pub layer: usize,
    pub sequence_changed: bool,
    pub error: Option<String>,
}
impl Draft {
    pub fn get(&self, id: &str) -> &str {
        self.fields
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }
    pub fn set(&mut self, id: &str, value: String) -> Result<()> {
        let f = self
            .fields
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or(Error::InvalidSelection)?;
        f.value = value;
        self.error = None;
        Ok(())
    }
    pub fn dirty(&self) -> bool {
        self.sequence_changed || self.node.is_none() || self.fields.iter().any(Field::changed)
    }
    pub fn positions(&self) -> Vec<usize> {
        self.get("key-positions")
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect()
    }
    pub fn toggle_position(&mut self, position: usize) {
        if self.section != Section::Combos {
            return;
        }
        let mut positions = self.positions();
        if let Some(index) = positions.iter().position(|p| *p == position) {
            positions.remove(index);
        } else {
            positions.push(position);
        }
        let _ = self.set(
            "key-positions",
            positions
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    pub fn add_step(&mut self) {
        self.sequence_changed = true;
        self.error = None;
        let index = self
            .fields
            .iter()
            .filter(|f| f.id.starts_with("step-mode-"))
            .count();
        self.fields.push(Field::new(
            &format!("step-mode-{index}"),
            "Action",
            "Tap",
            Kind::Choice(vec![
                "Tap".into(),
                "Press".into(),
                "Release".into(),
                "Raw".into(),
            ]),
        ));
        self.fields.push(Field::new(
            &format!("step-bindings-{index}"),
            "Bindings",
            "&kp A",
            Kind::Bindings,
        ));
        if let Some(f) = self.fields.last_mut() {
            f.initial = String::new();
        }
    }
    pub fn move_step(&mut self, index: usize, delta: i32) -> Result<()> {
        let count = self
            .fields
            .iter()
            .filter(|f| f.id.starts_with("step-mode-"))
            .count();
        let target = index as i64 + i64::from(delta);
        if index >= count || target < 0 || target >= count as i64 {
            return Err(Error::InvalidSelection);
        }
        for prefix in ["step-mode-", "step-bindings-"] {
            let left = self.get(&format!("{prefix}{index}")).to_string();
            let right = self.get(&format!("{prefix}{target}")).to_string();
            self.set(&format!("{prefix}{index}"), right)?;
            self.set(&format!("{prefix}{target}"), left)?;
        }
        self.sequence_changed = true;
        self.error = None;
        Ok(())
    }
    pub fn remove_step(&mut self, index: usize) {
        self.sequence_changed = true;
        self.error = None;
        self.fields.retain(|f| {
            f.id != format!("step-mode-{index}") && f.id != format!("step-bindings-{index}")
        });
        for (prefix, label) in [("step-mode-", "Action"), ("step-bindings-", "Bindings")] {
            for (i, f) in self
                .fields
                .iter_mut()
                .filter(|f| f.id.starts_with(prefix))
                .enumerate()
            {
                f.id = format!("{prefix}{i}");
                f.label = label.into();
                f.initial = "\0".into();
            }
        }
    }
}
fn all_nodes<'a>(node: &'a Node, out: &mut Vec<&'a Node>) {
    for child in &node.children {
        out.push(child);
        all_nodes(child, out);
    }
}
pub fn entries(project: &Project, section: Section, layer: usize) -> Result<Vec<Entry>> {
    let Some(doc) = &project.zmk else {
        return Ok(vec![]);
    };
    let tree = doc.tree()?;
    let mut nodes = vec![];
    all_nodes(&tree, &mut nodes);
    if section == Section::Encoders {
        let keymap = tree
            .matching(&doc.source, "zmk,keymap")
            .ok_or(Error::InvalidSelection)?;
        let layers = keymap
            .children
            .iter()
            .filter(|n| n.property("bindings").is_some())
            .collect::<Vec<_>>();
        let count = layers
            .iter()
            .filter_map(|n| n.value(&doc.source, "sensor-bindings"))
            .filter_map(|v| zmk::binding_spans(v).ok())
            .map(|v| v.len())
            .max()
            .unwrap_or(0);
        return Ok((0..count)
            .map(|i| Entry {
                id: format!("sensor/{i}"),
                name: format!("Encoder {i}"),
                detail: format!(
                    "Layer {} sensor binding",
                    project
                        .keymap
                        .layers
                        .get(layer)
                        .map(|l| l.name.as_str())
                        .unwrap_or("")
                ),
            })
            .collect());
    }
    let (group, compatible) = section.group();
    let selected = nodes.iter().copied().filter(|n| match section {
        Section::Macros => n
            .value(&doc.source, "compatible")
            .is_some_and(|v| zmk::unquote(v).starts_with("zmk,behavior-macro")),
        Section::Behaviors => {
            n.path
                .rsplit_once('/')
                .is_some_and(|(parent, _)| parent.rsplit('/').next() == Some("behaviors"))
                && !n
                    .value(&doc.source, "compatible")
                    .is_some_and(|v| zmk::unquote(v).starts_with("zmk,behavior-macro"))
        }
        _ => nodes.iter().any(|parent| {
            parent.children.iter().any(|child| child.path == n.path)
                && (parent.name == group
                    || compatible.is_some_and(|c| parent.compatible(&doc.source, c)))
        }),
    });
    Ok(selected
        .map(|node| Entry {
            id: node.path.clone(),
            name: node.label.clone().unwrap_or_else(|| node.name.clone()),
            detail: match section {
                Section::Combos => format!(
                    "Positions {}",
                    node.value(&doc.source, "key-positions")
                        .map(zmk::cells)
                        .unwrap_or_default()
                ),
                Section::ConditionalLayers => format!(
                    "{} → {}",
                    node.value(&doc.source, "if-layers")
                        .map(zmk::cells)
                        .unwrap_or_default(),
                    node.value(&doc.source, "then-layer")
                        .map(zmk::cells)
                        .unwrap_or_default()
                ),
                _ => node
                    .value(&doc.source, "compatible")
                    .map(zmk::unquote)
                    .unwrap_or_default(),
            },
        })
        .collect())
}
fn prop(node: Option<&Node>, source: &str, name: &str, default: &str) -> String {
    if name == "hold-trigger-on-release" {
        return if node.is_some_and(|n| n.property(name).is_some()) {
            "Enabled"
        } else {
            "Disabled"
        }
        .into();
    }
    node.and_then(|n| n.value(source, name))
        .map(|s| {
            if s.starts_with('"') {
                zmk::unquote(s)
            } else {
                zmk::cells(s)
            }
        })
        .unwrap_or_else(|| default.into())
}
pub fn draft(project: &Project, section: Section, id: Option<&str>, layer: usize) -> Result<Draft> {
    draft_with_kind(project, section, id, layer, None)
}
pub fn new_behavior(project: &Project, kind: &str, layer: usize) -> Result<Draft> {
    if ![
        "zmk,behavior-hold-tap",
        "zmk,behavior-tap-dance",
        "zmk,behavior-mod-morph",
    ]
    .contains(&kind)
    {
        return Err(Error::InvalidDocument(
            "Choose a supported behavior type".into(),
        ));
    }
    draft_with_kind(project, Section::Behaviors, None, layer, Some(kind))
}
fn draft_with_kind(
    project: &Project,
    section: Section,
    id: Option<&str>,
    layer: usize,
    behavior_kind: Option<&str>,
) -> Result<Draft> {
    let doc = project
        .zmk
        .as_ref()
        .ok_or_else(|| Error::InvalidDocument("Open a ZMK keymap first".into()))?;
    let tree = doc.tree()?;
    let node = if section == Section::Encoders {
        None
    } else {
        id.and_then(|id| tree.find(id))
    };
    if id.is_some() && section != Section::Encoders && node.is_none() {
        return Err(Error::InvalidSelection);
    }
    let mut fields = vec![];
    if section != Section::Encoders {
        fields.push(Field::new(
            "name",
            "Name",
            node.map(|n| n.label.as_deref().unwrap_or(&n.name))
                .unwrap_or(""),
            Kind::Text,
        ));
    }
    let mut add = |id: &str, label: &str, default: &str, kind: Kind| {
        fields.push(Field::new(
            id,
            label,
            prop(node, &doc.source, id, default),
            kind,
        ))
    };
    match section {
        Section::Combos => {
            add("key-positions", "Key positions", "", Kind::Positions);
            add("bindings", "Result binding", "&kp ESC", Kind::Binding);
            add("timeout-ms", "Timeout (ms)", "50", Kind::Number);
            add(
                "layers",
                "Active layers (empty means all)",
                "",
                Kind::Layers,
            );
        }
        Section::Macros => {
            add("wait-ms", "Wait time (ms)", "15", Kind::Number);
            add("tap-ms", "Tap time (ms)", "30", Kind::Number);
            let raw = node
                .and_then(|n| n.value(&doc.source, "bindings"))
                .unwrap_or("<&macro_tap &kp A>");
            let ts = tokens_groups(raw);
            for (i, raw) in ts.iter().enumerate() {
                let (mode, bindings) = if let Some(v) = raw.strip_prefix("&macro_press ") {
                    ("Press", v)
                } else if let Some(v) = raw.strip_prefix("&macro_release ") {
                    ("Release", v)
                } else if let Some(v) = raw.strip_prefix("&macro_tap ") {
                    ("Tap", v)
                } else {
                    ("Raw", raw.as_str())
                };
                fields.push(Field::new(
                    &format!("step-mode-{i}"),
                    "Action",
                    mode,
                    Kind::Choice(vec![
                        "Tap".into(),
                        "Press".into(),
                        "Release".into(),
                        "Raw".into(),
                    ]),
                ));
                fields.push(Field::new(
                    &format!("step-bindings-{i}"),
                    "Bindings",
                    bindings,
                    Kind::Bindings,
                ));
            }
        }
        Section::Behaviors => {
            let kind = node
                .and_then(|n| n.value(&doc.source, "compatible"))
                .map(zmk::unquote)
                .unwrap_or_else(|| behavior_kind.unwrap_or("zmk,behavior-hold-tap").into());
            if node.is_none() {
                add(
                    "behavior-type",
                    "Behavior type",
                    &kind,
                    Kind::Choice(vec![
                        "zmk,behavior-hold-tap".into(),
                        "zmk,behavior-tap-dance".into(),
                        "zmk,behavior-mod-morph".into(),
                    ]),
                );
            }
            if kind == "zmk,behavior-hold-tap" {
                add(
                    "flavor",
                    "Flavor",
                    "balanced",
                    Kind::Choice(vec![
                        "balanced".into(),
                        "tap-preferred".into(),
                        "hold-preferred".into(),
                        "tap-unless-interrupted".into(),
                    ]),
                );
                add("tapping-term-ms", "Tapping term (ms)", "200", Kind::Number);
                add("quick-tap-ms", "Quick tap (ms)", "175", Kind::Number);
                add(
                    "hold-trigger-on-release",
                    "Hold trigger on release",
                    "Disabled",
                    Kind::Choice(vec!["Enabled".into(), "Disabled".into()]),
                );
                add(
                    "hold-trigger-key-positions",
                    "Hold trigger positions (optional)",
                    "",
                    Kind::Positions,
                );
            }
            if kind == "zmk,behavior-tap-dance" {
                add("tapping-term-ms", "Tapping term (ms)", "200", Kind::Number);
            }
            if kind == "zmk,behavior-mod-morph" {
                add("mods", "Modifier mask", "(MOD_LSFT | MOD_RSFT)", Kind::Text);
            }
            if [
                "zmk,behavior-hold-tap",
                "zmk,behavior-tap-dance",
                "zmk,behavior-mod-morph",
            ]
            .contains(&kind.as_str())
                || node.is_some_and(|n| n.property("bindings").is_some())
            {
                add(
                    "bindings",
                    "Behavior references / bindings",
                    match kind.as_str() {
                        "zmk,behavior-tap-dance" => "&kp A &kp B",
                        "zmk,behavior-mod-morph" => "&kp SEMI &kp COLON",
                        _ => "&kp &kp",
                    },
                    Kind::Bindings,
                );
            }
        }
        Section::ConditionalLayers => {
            add(
                "if-layers",
                "If all these layers are active",
                "1 2",
                Kind::Layers,
            );
            add(
                "then-layer",
                "Activate layer",
                if project.keymap.layers.len() > 3 {
                    "3"
                } else {
                    "0"
                },
                Kind::Layer,
            );
        }
        Section::Encoders => {
            let index = id
                .and_then(|v| v.strip_prefix("sensor/"))
                .and_then(|v| v.parse::<usize>().ok())
                .ok_or(Error::InvalidSelection)?;
            let keymap = tree
                .matching(&doc.source, "zmk,keymap")
                .ok_or(Error::InvalidSelection)?;
            let node = keymap
                .children
                .iter()
                .filter(|n| n.property("bindings").is_some())
                .nth(layer)
                .ok_or(Error::InvalidSelection)?;
            let binding = node
                .value(&doc.source, "sensor-bindings")
                .and_then(|v| {
                    zmk::binding_spans(v)
                        .ok()
                        .and_then(|spans| spans.get(index).map(|s| v[s.clone()].to_owned()))
                })
                .unwrap_or_else(|| "&inc_dec_kp C_VOL_UP C_VOL_DN".into());
            let parameters = binding.split_whitespace().collect::<Vec<_>>();
            if let ["&inc_dec_kp", clockwise, counterclockwise] = parameters.as_slice() {
                let mut choices = crate::supported_key_codes();
                for value in [clockwise, counterclockwise] {
                    if !choices.iter().any(|v| v == value) {
                        choices.push(value.to_string());
                    }
                }
                fields.push(Field::new(
                    "encoder-clockwise",
                    "Clockwise keycode",
                    *clockwise,
                    Kind::Choice(choices.clone()),
                ));
                fields.push(Field::new(
                    "encoder-counterclockwise",
                    "Counterclockwise keycode",
                    *counterclockwise,
                    Kind::Choice(choices),
                ));
            } else {
                fields.push(Field::new(
                    "sensor-binding",
                    "Rotation binding (clockwise, counterclockwise)",
                    binding,
                    Kind::Binding,
                ));
            }
        }
    }
    Ok(Draft {
        section,
        node: id.map(str::to_string),
        fields,
        layer,
        sequence_changed: false,
        error: None,
    })
}
fn tokens_groups(value: &str) -> Vec<String> {
    let mut groups = vec![];
    let mut start = None;
    for (i, c) in value.char_indices() {
        if c == '<' {
            start = Some(i + 1);
        } else if c == '>'
            && let Some(start) = start.take()
        {
            groups.push(value[start..i].trim().into());
        }
    }
    if groups.is_empty() && !value.trim().is_empty() {
        groups.push(zmk::cells(value));
    }
    groups
}
fn integers(value: &str, limit: usize) -> Result<Vec<usize>> {
    let numbers = value
        .split_whitespace()
        .map(|v| {
            v.parse::<usize>().map_err(|_| {
                Error::InvalidDocument(
                    "Use space-separated numeric positions or layer numbers".into(),
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if numbers.iter().any(|v| *v >= limit) {
        return Err(Error::InvalidDocument(format!(
            "Values must be smaller than {limit}"
        )));
    }
    let mut unique = numbers.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != numbers.len() {
        return Err(Error::InvalidDocument(
            "Duplicate positions or layers".into(),
        ));
    }
    Ok(numbers)
}
pub fn apply(project: &Project, draft: &Draft) -> Result<Document> {
    let mut doc = project.zmk.clone().ok_or(Error::InvalidSelection)?;
    let new = draft.node.is_none();
    let mut properties = BTreeMap::new();
    for field in &draft.fields {
        if field.id == "name"
            || field.id == "behavior-type"
            || field.id.starts_with("step-")
            || field.id.starts_with("encoder-")
        {
            continue;
        }
        if !new && !field.changed() {
            continue;
        }
        let value = field.value.trim();
        let optional = ["layers", "hold-trigger-key-positions"].contains(&field.id.as_str());
        if optional && value.is_empty() {
            properties.insert(field.id.clone(), None);
            continue;
        }
        match &field.kind {
            Kind::Number => {
                let n = value.parse::<u32>().map_err(|_| {
                    Error::InvalidDocument(format!(
                        "{} must be a non-negative integer",
                        field.label
                    ))
                })?;
                if n > 60000 {
                    return Err(Error::InvalidDocument(
                        "Timing must be at most 60000 ms".into(),
                    ));
                }
            }
            Kind::Positions => {
                let numbers = integers(value, project.keyboard.keys.len())?;
                if draft.section == Section::Combos && numbers.len() < 2 {
                    return Err(Error::InvalidDocument(
                        "A combo needs at least two positions".into(),
                    ));
                }
            }
            Kind::Layers | Kind::Layer => {
                let values = integers(value, project.keymap.layers.len())?;
                if (matches!(field.kind, Kind::Layer) && values.len() != 1)
                    || (!optional && values.is_empty())
                {
                    return Err(Error::InvalidDocument("Choose the required layers".into()));
                }
            }
            Kind::Binding => {
                zmk::binding(value)?;
            }
            Kind::Bindings => {
                if zmk::binding_spans(value)?.is_empty() {
                    return Err(Error::InvalidDocument(
                        "Enter at least one behavior reference".into(),
                    ));
                }
            }
            Kind::Choice(choices) => {
                if !choices.iter().any(|c| c == value) {
                    return Err(Error::InvalidDocument("Choose a supported value".into()));
                }
            }
            Kind::Text => {}
        }
        if field.id == "hold-trigger-on-release" {
            properties.insert(
                field.id.clone(),
                if value == "Enabled" {
                    Some(String::new())
                } else {
                    None
                },
            );
            continue;
        }
        properties.insert(
            field.id.clone(),
            Some(if field.id == "flavor" {
                zmk::quote(value)
            } else {
                format!("<{value}>")
            }),
        );
    }
    if draft.section == Section::Macros
        && (draft.sequence_changed
            || new
            || draft
                .fields
                .iter()
                .any(|f| f.id.starts_with("step-") && f.changed()))
    {
        let count = draft
            .fields
            .iter()
            .filter(|f| f.id.starts_with("step-mode-"))
            .count();
        if count == 0 {
            return Err(Error::InvalidDocument(
                "A macro needs a sequence step".into(),
            ));
        }
        let mut sequence = vec![];
        for i in 0..count {
            let mode = draft.get(&format!("step-mode-{i}"));
            let value = draft.get(&format!("step-bindings-{i}")).trim();
            if zmk::binding_spans(value)?.is_empty() {
                return Err(Error::InvalidDocument(
                    "Each step needs at least one binding".into(),
                ));
            }
            let prefix = match mode {
                "Tap" => "&macro_tap ",
                "Press" => "&macro_press ",
                "Release" => "&macro_release ",
                "Raw" => "",
                _ => return Err(Error::InvalidSelection),
            };
            sequence.push(format!("<{prefix}{value}>"));
        }
        properties.insert(
            "bindings".into(),
            Some(sequence.join(",\n                ")),
        );
    }
    if draft.section == Section::ConditionalLayers {
        let ifs = draft
            .get("if-layers")
            .split_whitespace()
            .collect::<Vec<_>>();
        if ifs.contains(&draft.get("then-layer").trim()) {
            return Err(Error::InvalidDocument(
                "The activated layer cannot require itself".into(),
            ));
        }
    }
    if draft.section == Section::Encoders {
        let index = draft
            .node
            .as_deref()
            .and_then(|s| s.strip_prefix("sensor/"))
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or(Error::InvalidSelection)?;
        let tree = doc.tree()?;
        let keymap = tree
            .matching(&doc.source, "zmk,keymap")
            .ok_or(Error::InvalidSelection)?;
        let layer = keymap
            .children
            .iter()
            .filter(|n| n.property("bindings").is_some())
            .nth(draft.layer)
            .ok_or(Error::InvalidSelection)?;
        let path = layer.path.clone();
        let raw = layer.value(&doc.source, "sensor-bindings").unwrap_or("");
        let mut bindings = zmk::binding_spans(raw)?
            .into_iter()
            .map(|span| raw[span].to_owned())
            .collect::<Vec<_>>();
        while bindings.len() <= index {
            bindings.push("&inc_dec_kp C_VOL_UP C_VOL_DN".into());
        }
        bindings[index] = if draft.fields.iter().any(|f| f.id == "encoder-clockwise") {
            let clockwise = draft.get("encoder-clockwise");
            let counterclockwise = draft.get("encoder-counterclockwise");
            if !crate::valid_key_code(clockwise) || !crate::valid_key_code(counterclockwise) {
                return Err(Error::InvalidDocument(
                    "Choose valid encoder keycodes".into(),
                ));
            }
            format!("&inc_dec_kp {clockwise} {counterclockwise}")
        } else {
            draft.get("sensor-binding").trim().into()
        };
        let mut values = BTreeMap::new();
        values.insert(
            "sensor-bindings".into(),
            Some(format!("<{}>", bindings.join(" "))),
        );
        doc.set_properties(&path, &values)?;
    } else if let Some(path) = &draft.node {
        doc.set_properties(path, &properties)?;
        if draft.fields.iter().any(|f| f.id == "name" && f.changed()) {
            doc.rename_node(path, draft.get("name").trim())?;
        }
    } else {
        let name = draft.get("name").trim();
        let (group, compatible) = draft.section.group();
        let mut body = String::new();
        if draft.section == Section::Macros {
            body.push_str("            compatible = \"zmk,behavior-macro\";\n            #binding-cells = <0>;\n");
        }
        if draft.section == Section::Behaviors {
            let kind = draft.get("behavior-type");
            if ![
                "zmk,behavior-hold-tap",
                "zmk,behavior-tap-dance",
                "zmk,behavior-mod-morph",
            ]
            .contains(&kind)
            {
                return Err(Error::InvalidDocument(
                    "Choose a supported behavior type".into(),
                ));
            }
            body.push_str(&format!(
                "            compatible = {};\n            #binding-cells = <{}>;\n",
                zmk::quote(kind),
                if kind == "zmk,behavior-hold-tap" {
                    2
                } else {
                    0
                }
            ));
        }

        for (name, value) in properties {
            if let Some(value) = value {
                body.push_str(&if value.is_empty() {
                    format!("            {name};\n")
                } else {
                    format!("            {name} = {value};\n")
                });
            }
        }
        doc.add_node(
            group,
            compatible,
            name,
            &body,
            matches!(draft.section, Section::Macros | Section::Behaviors),
        )?;
    }
    doc.layers()?;
    Ok(doc)
}
pub fn delete(project: &Project, draft: &Draft) -> Result<Document> {
    let mut doc = project.zmk.clone().ok_or(Error::InvalidSelection)?;
    let path = draft.node.as_deref().ok_or(Error::InvalidSelection)?;
    if draft.section == Section::Encoders {
        return Err(Error::InvalidDocument(
            "Encoder availability belongs to the hardware configuration".into(),
        ));
    }
    let tree = doc.tree()?;
    let node = tree.find(path).ok_or(Error::InvalidSelection)?;
    if let Some(label) = &node.label {
        let mut outside = doc.source.clone();
        outside.replace_range(node.span.clone(), "");
        if outside.contains(&format!("&{label}")) {
            return Err(Error::InvalidDocument(
                "This definition is still referenced by the keymap. Remove its bindings first."
                    .into(),
            ));
        }
    }
    doc.remove_node(path)?;
    Ok(doc)
}
