//! Source-preserving Devicetree editing. Untouched bytes, comments and includes
//! remain intact; this is intentionally not a C preprocessor or firmware compiler.
use crate::{Binding, Error, KeyCode, Layer, LayerId, Project, Result, layout::Keyboard};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Range};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin {
    Local {
        path: std::path::PathBuf,
        saved_source: String,
    },
    GitHub {
        repository: String,
        branch: String,
        path: String,
        sha: String,
        commit: String,
        #[serde(default)]
        workflow: Option<u64>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub source: String,
    pub origin: Origin,
}
#[derive(Debug, Clone)]
pub struct Property {
    pub name: String,
    pub value: Range<usize>,
    pub statement: Range<usize>,
}
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub label: Option<String>,
    pub path: String,
    pub span: Range<usize>,
    pub close: usize,
    pub properties: Vec<Property>,
    pub children: Vec<Node>,
}
impl Node {
    pub fn property(&self, name: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.name == name)
    }
    pub fn value<'a>(&self, source: &'a str, name: &str) -> Option<&'a str> {
        self.property(name).map(|p| source[p.value.clone()].trim())
    }
    pub fn find(&self, path: &str) -> Option<&Node> {
        if self.path == path {
            Some(self)
        } else {
            self.children.iter().find_map(|n| n.find(path))
        }
    }
    pub fn compatible(&self, source: &str, kind: &str) -> bool {
        self.value(source, "compatible")
            .is_some_and(|v| unquote(v) == kind)
    }
    pub fn matching<'a>(&'a self, source: &str, kind: &str) -> Option<&'a Node> {
        if self.compatible(source, kind) {
            Some(self)
        } else {
            self.children.iter().find_map(|n| n.matching(source, kind))
        }
    }
}
#[derive(Debug, Clone)]
struct Token {
    text: String,
    span: Range<usize>,
}
fn invalid(message: impl Into<String>) -> Error {
    Error::InvalidDocument(message.into())
}

fn tokens(source: &str) -> Result<Vec<Token>> {
    let b = source.as_bytes();
    let mut i = 0;
    let mut out = vec![];
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if b[i..].starts_with(b"//") {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b[i..].starts_with(b"/*") {
            let start = i;
            i += 2;
            while i + 1 < b.len() && !b[i..].starts_with(b"*/") {
                i += 1;
            }
            if i + 1 >= b.len() {
                return Err(invalid(format!("Unclosed comment at byte {start}")));
            }
            i += 2;
            continue;
        }
        if b[i] == b'#' {
            let end = source[i..].find('\n').map(|n| i + n).unwrap_or(b.len());
            let directive = source[i..end].split_whitespace().next().unwrap_or("");
            if ["#if", "#ifdef", "#ifndef", "#elif", "#else", "#endif"].contains(&directive) {
                return Err(invalid(
                    "Conditional preprocessing requires a resolved keymap. This file is left unchanged; choose its resolved .keymap file.",
                ));
            }
            if ["#include", "#define", "#undef", "#pragma"].contains(&directive) {
                i = end;
                continue;
            }
        }
        let start = i;
        if b[i] == b'"' {
            i += 1;
            let mut closed = false;
            while i < b.len() {
                if b[i] == b'\\' {
                    i = (i + 2).min(b.len());
                } else if b[i] == b'"' {
                    i += 1;
                    closed = true;
                    break;
                } else {
                    i += 1;
                }
            }
            if !closed {
                return Err(invalid("Unclosed string"));
            }
        } else if b"{};=<>,:[]".contains(&b[i]) {
            i += 1;
        } else {
            while i < b.len()
                && !b[i].is_ascii_whitespace()
                && !b"{};=<>,:[]\"".contains(&b[i])
                && !b[i..].starts_with(b"//")
                && !b[i..].starts_with(b"/*")
            {
                i += 1;
            }
        }
        if i == start {
            return Err(invalid("Unrecognized source token"));
        }
        out.push(Token {
            text: source[start..i].to_owned(),
            span: start..i,
        });
    }
    Ok(out)
}
fn parse_body(
    ts: &[Token],
    i: &mut usize,
    path: &str,
    name: String,
    label: Option<String>,
    start: usize,
    length: usize,
) -> Result<Node> {
    let mut node = Node {
        name,
        label,
        path: path.into(),
        span: start..length,
        close: length,
        properties: vec![],
        children: vec![],
    };
    while *i < ts.len() {
        if ts[*i].text == "}" {
            node.close = ts[*i].span.start;
            *i += 1;
            node.span.end = ts[*i - 1].span.end;
            if ts.get(*i).is_some_and(|t| t.text == ";") {
                node.span.end = ts[*i].span.end;
                *i += 1;
            }
            return Ok(node);
        }
        let begin = *i;
        while *i < ts.len() && !["{", ";", "}"].contains(&ts[*i].text.as_str()) {
            *i += 1;
        }
        if *i == ts.len() {
            return Err(invalid("Unterminated Devicetree statement"));
        }
        match ts[*i].text.as_str() {
            "{" => {
                let header = &ts[begin..*i];
                let child_name = header
                    .last()
                    .ok_or_else(|| invalid("Missing node name"))?
                    .text
                    .clone();
                let label = header
                    .iter()
                    .position(|t| t.text == ":")
                    .and_then(|n| n.checked_sub(1))
                    .map(|n| header[n].text.clone());
                let child_path = if child_name == "/" && path.is_empty() {
                    "/".into()
                } else {
                    format!("{}/{child_name}", path.trim_end_matches('/'))
                };
                let start = ts[begin].span.start;
                *i += 1;
                node.children.push(parse_body(
                    ts,
                    i,
                    &child_path,
                    child_name,
                    label,
                    start,
                    length,
                )?);
            }
            ";" => {
                let stmt = &ts[begin..*i];
                if let Some(first) = stmt.first() {
                    let value = if let Some(eq) = stmt.iter().position(|t| t.text == "=") {
                        if eq + 1 >= stmt.len() {
                            return Err(invalid("Missing property value"));
                        }
                        stmt[eq + 1].span.start..stmt.last().unwrap().span.end
                    } else {
                        ts[*i].span.start..ts[*i].span.start
                    };
                    node.properties.push(Property {
                        name: first.text.clone(),
                        value,
                        statement: first.span.start..ts[*i].span.end,
                    });
                }
                *i += 1;
            }
            _ => return Err(invalid("Unexpected closing brace")),
        }
    }
    if path.is_empty() {
        Ok(node)
    } else {
        Err(invalid("Unclosed Devicetree node"))
    }
}
pub fn parse(source: &str) -> Result<Node> {
    let ts = tokens(source)?;
    let mut i = 0;
    parse_body(&ts, &mut i, "", String::new(), None, 0, source.len())
}
pub fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}
pub fn cells(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim()
        .into()
}
pub fn quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace(['\n', '\r'], " ")
    )
}
pub fn identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        && !name.as_bytes()[0].is_ascii_digit()
}

/// Each reference and its parameters keep their own source span. Comments and
/// whitespace between references are excluded from a replacement.
pub fn binding_spans(value: &str) -> Result<Vec<Range<usize>>> {
    let ts = tokens(value)?;
    let mut spans: Vec<Range<usize>> = vec![];
    let mut parens = 0i32;
    for t in ts {
        if ["<", ">", ","].contains(&t.text.as_str()) {
            continue;
        }
        if t.text.starts_with('&') && parens == 0 {
            if !identifier(&t.text[1..]) {
                return Err(invalid("Invalid behavior reference"));
            }
            spans.push(t.span.clone());
        } else if let Some(last) = spans.last_mut() {
            last.end = t.span.end;
        } else {
            return Err(invalid("A binding must start with &behavior"));
        }
        parens += t.text.bytes().filter(|b| *b == b'(').count() as i32
            - t.text.bytes().filter(|b| *b == b')').count() as i32;
        if parens < 0 {
            return Err(invalid("Unbalanced binding expression"));
        }
    }
    if parens != 0 {
        return Err(invalid("Unbalanced binding expression"));
    }
    Ok(spans)
}
pub fn binding(value: &str) -> Result<Binding> {
    let value = value.trim();
    if value.contains(['{', '}', ';', '<', '>', '"', ',']) {
        return Err(invalid(
            "Enter a single binding without Devicetree delimiters",
        ));
    }
    let spans = binding_spans(value)?;
    if spans.len() != 1 {
        return Err(invalid("Expected one behavior binding"));
    }
    let parts: Vec<_> = value.split_whitespace().collect();
    Ok(match parts.as_slice() {
        ["&kp", code] => Binding::KeyPress(KeyCode((*code).into())),
        ["&mt", hold, tap] if crate::MODIFIERS.contains(hold) => Binding::ModTap {
            hold: KeyCode((*hold).into()),
            tap: KeyCode((*tap).into()),
        },
        ["&lt", layer, tap] if layer.parse::<usize>().is_ok() => Binding::LayerTap {
            layer: LayerId(layer.parse().unwrap()),
            tap: KeyCode((*tap).into()),
        },
        ["&mo", layer] if layer.parse::<usize>().is_ok() => {
            Binding::MomentaryLayer(LayerId(layer.parse().unwrap()))
        }
        ["&trans"] => Binding::Transparent,
        ["&none"] => Binding::None,
        _ => Binding::Custom(value.into()),
    })
}
impl Document {
    /// Store the user-facing name without changing node identity or layer indices.
    pub fn rename_layer(&mut self, layer: usize, name: &str) -> Result<()> {
        let tree = self.tree()?;
        let keymap = tree
            .matching(&self.source, "zmk,keymap")
            .ok_or_else(|| invalid("No keymap node"))?;
        let node = keymap
            .children
            .iter()
            .filter(|n| n.property("bindings").is_some())
            .nth(layer)
            .ok_or(Error::InvalidSelection)?;
        let path = node.path.clone();
        self.set_properties(
            &path,
            &BTreeMap::from([("display-name".into(), Some(quote(name)))]),
        )
    }
    /// Resolve hold/tap child behaviors from the loaded source, without relying
    /// on a user's chosen behavior name (for example HML or HMR).
    pub fn hold_tap_behaviors(&self) -> Result<BTreeMap<String, [String; 2]>> {
        let mut behaviors = BTreeMap::from([
            ("&mt".into(), ["&kp".into(), "&kp".into()]),
            ("&lt".into(), ["&mo".into(), "&kp".into()]),
        ]);
        fn collect(
            node: &Node,
            source: &str,
            behaviors: &mut BTreeMap<String, [String; 2]>,
        ) -> Result<()> {
            if node.compatible(source, "zmk,behavior-hold-tap")
                && let (Some(label), Some(value)) = (&node.label, node.value(source, "bindings"))
            {
                let spans = binding_spans(value)?;
                let children: Vec<_> = spans
                    .iter()
                    .map(|span| value[span.clone()].trim())
                    .collect();
                if let [hold, tap] = children.as_slice()
                    && hold.split_whitespace().count() == 1
                    && tap.split_whitespace().count() == 1
                {
                    behaviors.insert(format!("&{label}"), [(*hold).into(), (*tap).into()]);
                }
            }
            for child in &node.children {
                collect(child, source, behaviors)?;
            }
            Ok(())
        }
        collect(&self.tree()?, &self.source, &mut behaviors)?;
        Ok(behaviors)
    }
    pub fn tree(&self) -> Result<Node> {
        parse(&self.source)
    }
    pub fn layers(&self) -> Result<Vec<Layer>> {
        let tree = self.tree()?;
        let keymap = tree
            .matching(&self.source, "zmk,keymap")
            .ok_or_else(|| invalid("No zmk,keymap node in this file"))?;
        let mut layers = vec![];
        for node in &keymap.children {
            if let Some(prop) = node.property("bindings") {
                let raw = &self.source[prop.value.clone()];
                let bindings = binding_spans(raw)?
                    .into_iter()
                    .map(|span| binding(&raw[span]))
                    .collect::<Result<Vec<_>>>()?;
                if bindings.is_empty() {
                    return Err(invalid(format!(
                        "Layer {} has no resolvable bindings",
                        node.name
                    )));
                }
                layers.push(Layer {
                    name: node
                        .value(&self.source, "display-name")
                        .map(unquote)
                        .unwrap_or_else(|| node.name.clone()),
                    bindings,
                });
            }
        }
        if layers.is_empty() {
            return Err(invalid("The keymap contains no editable layers"));
        }
        Ok(layers)
    }
    pub fn project(&self, keyboard: Keyboard) -> Result<Project> {
        let layers = self.layers()?;
        if layers
            .iter()
            .any(|l| l.bindings.len() != keyboard.keys.len())
        {
            return Err(invalid(format!(
                "This keymap has {} positions; the available {} geometry has {}. A matching physical layout is required.",
                layers[0].bindings.len(),
                keyboard.name,
                keyboard.keys.len()
            )));
        }
        let (repository, path) = match &self.origin {
            Origin::GitHub {
                repository,
                branch,
                path,
                ..
            } => {
                let (owner, name) = repository
                    .split_once('/')
                    .ok_or_else(|| invalid("Invalid repository"))?;
                (
                    crate::Repository {
                        owner: owner.into(),
                        name: name.into(),
                        branch: branch.clone(),
                    },
                    path.clone(),
                )
            }
            Origin::Local { path, .. } => (
                crate::Repository {
                    owner: "Local".into(),
                    name: "file".into(),
                    branch: String::new(),
                },
                path.to_string_lossy().into(),
            ),
        };
        let project = Project {
            schema_version: 1,
            name: path.rsplit('/').next().unwrap_or(&path).into(),
            repository,
            configuration: crate::Configuration {
                board: String::new(),
                shields: vec![],
            },
            keyboard,
            keymap: crate::Keymap { layers },
            zmk: Some(self.clone()),
        };
        project.check_structure()?;
        Ok(project)
    }
    pub fn replace_binding(&mut self, layer: usize, key: usize, value: &str) -> Result<()> {
        binding(value)?;
        let tree = self.tree()?;
        let keymap = tree
            .matching(&self.source, "zmk,keymap")
            .ok_or_else(|| invalid("Missing keymap"))?;
        let node = keymap
            .children
            .iter()
            .filter(|n| n.property("bindings").is_some())
            .nth(layer)
            .ok_or(Error::InvalidSelection)?;
        let prop = node.property("bindings").unwrap();
        let spans = binding_spans(&self.source[prop.value.clone()])?;
        let span = spans.get(key).ok_or(Error::InvalidSelection)?;
        self.source.replace_range(
            prop.value.start + span.start..prop.value.start + span.end,
            value,
        );
        Ok(())
    }
    pub fn set_properties(
        &mut self,
        path: &str,
        values: &BTreeMap<String, Option<String>>,
    ) -> Result<()> {
        let tree = self.tree()?;
        let node = tree.find(path).ok_or(Error::InvalidSelection)?;
        let mut replacements = vec![];
        let mut extra = String::new();
        for (name, value) in values {
            if let Some(prop) = node.property(name) {
                match value {
                    Some(v) => replacements.push((prop.value.clone(), v.clone())),
                    None => replacements.push((prop.statement.clone(), String::new())),
                }
            } else if let Some(value) = value {
                extra.push_str(&if value.is_empty() {
                    format!("\n            {name};")
                } else {
                    format!("\n            {name} = {value};")
                });
            }
        }
        if !extra.is_empty() {
            extra.push('\n');
            replacements.push((node.close..node.close, extra));
        }
        replacements.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
        let mut source = self.source.clone();
        for (span, value) in replacements {
            source.replace_range(span, &value);
        }
        parse(&source)?;
        self.source = source;
        Ok(())
    }
    /// Rename a definition and direct phandle references in this source file.
    pub fn rename_node(&mut self, path: &str, name: &str) -> Result<()> {
        if !identifier(name) {
            return Err(invalid("Enter a valid definition name"));
        }
        let tree = self.tree()?;
        let node = tree.find(path).ok_or(Error::InvalidSelection)?;
        let old = node.label.as_deref().unwrap_or(&node.name);
        if old == name {
            return Ok(());
        }
        fn collect<'a>(node: &'a Node, out: &mut Vec<&'a Node>) {
            out.push(node);
            for child in &node.children {
                collect(child, out);
            }
        }
        let mut nodes = vec![];
        collect(&tree, &mut nodes);
        let parent = path
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .unwrap_or("");
        if nodes.iter().any(|other| {
            other.path != path
                && ((node.label.is_some() && other.label.as_deref() == Some(name))
                    || (other
                        .path
                        .rsplit_once('/')
                        .is_some_and(|(p, _)| p == parent)
                        && other.name == name))
        }) {
            return Err(invalid("That definition name already exists"));
        }
        let reference = format!("&{old}");
        if node.label.is_some()
            && self
                .source
                .lines()
                .any(|line| line.trim_start().starts_with("#define") && line.contains(&reference))
        {
            return Err(invalid(
                "This definition is referenced by a preprocessor macro. Its name is preserved; edit its parameters instead.",
            ));
        }
        let mut replacements = vec![];
        for token in tokens(&self.source)? {
            if token.span.start >= node.span.start
                && token.span.end <= node.close
                && token.text == "{"
            {
                break;
            }
            if token.span.start >= node.span.start
                && token.span.end <= node.close
                && token.text == old
            {
                replacements.push((token.span, name.to_string()));
            }
        }
        if node.label.is_some() {
            for token in tokens(&self.source)? {
                if token.text == reference {
                    replacements.push((token.span, format!("&{name}")));
                }
            }
            if node
                .value(&self.source, "label")
                .is_some_and(|v| unquote(v) == old)
                && let Some(property) = node.property("label")
            {
                replacements.push((property.value.clone(), quote(name)));
            }
        }
        replacements.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
        let mut source = self.source.clone();
        for (span, value) in replacements {
            source.replace_range(span, &value);
        }
        parse(&source)?;
        self.source = source;
        Ok(())
    }
    pub fn remove_node(&mut self, path: &str) -> Result<()> {
        let tree = self.tree()?;
        let node = tree.find(path).ok_or(Error::InvalidSelection)?;
        self.source.replace_range(node.span.clone(), "");
        Ok(())
    }
    pub fn add_node(
        &mut self,
        group: &str,
        compatible: Option<&str>,
        name: &str,
        body: &str,
        labeled: bool,
    ) -> Result<String> {
        if !identifier(name) {
            return Err(invalid(
                "Use a name starting with a letter or underscore, containing letters, digits, _ or -",
            ));
        }
        let tree = self.tree()?;
        let root = tree
            .children
            .iter()
            .find(|n| n.name == "/")
            .ok_or_else(|| invalid("Missing root node"))?;
        let header = if labeled {
            format!("{name}: {name}")
        } else {
            name.into()
        };
        let definition = format!("\n        {header} {{\n{body}\n        }};\n");
        if let Some(parent) = root.children.iter().find(|n| n.name == group) {
            if parent
                .children
                .iter()
                .any(|n| n.name == name || n.label.as_deref() == Some(name))
            {
                return Err(invalid("That name already exists"));
            }
            let path = format!("{}/{name}", parent.path);
            self.source.insert_str(parent.close, &definition);
            Ok(path)
        } else {
            let compatible = compatible
                .map(|c| format!("\n        compatible = {};", quote(c)))
                .unwrap_or_default();
            let path = format!("{}/{group}/{name}", root.path.trim_end_matches('/'));
            self.source.insert_str(
                root.close,
                &format!("\n    {group} {{{compatible}{definition}    }};\n"),
            );
            Ok(path)
        }
    }
}

/// Presentation consumers can show both meanings while the original binding
/// remains authoritative and is never normalized or rewritten by this lookup.
pub fn resolved_hold_tap(
    value: &Binding,
    behaviors: &BTreeMap<String, [String; 2]>,
) -> Option<(Binding, Binding)> {
    match value {
        Binding::ModTap { hold, tap } => Some((
            Binding::KeyPress(hold.clone()),
            Binding::KeyPress(tap.clone()),
        )),
        Binding::LayerTap { layer, tap } => Some((
            Binding::MomentaryLayer(*layer),
            Binding::KeyPress(tap.clone()),
        )),
        Binding::Custom(raw) => {
            let parts: Vec<_> = raw.split_whitespace().collect();
            let [name, hold, tap] = parts.as_slice() else {
                return None;
            };
            let [hold_behavior, tap_behavior] = behaviors.get(*name)?;
            Some((
                binding(&format!("{hold_behavior} {hold}")).ok()?,
                binding(&format!("{tap_behavior} {tap}")).ok()?,
            ))
        }
        _ => None,
    }
}
