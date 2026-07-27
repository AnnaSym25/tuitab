//! Document tree — the shared in-memory model for JSON / JSONL / YAML / TOML.
//!
//! All four formats parse into one [`Node`] tree.  The table shown on screen is a
//! *projection* of a subtree (see [`crate::data::view`]), not a copy: editing a cell
//! writes back into the tree by [`NodePath`], and saving re-serialises the tree into
//! whichever format the target file asks for.  Cross-format conversion therefore falls
//! out for free — `config.toml` can be saved as `config.yaml` with no extra code.
//!
//! Known ceilings, all deliberate:
//! - datetimes are carried as RFC-3339 text so TOML round-trips exactly; converting to
//!   JSON emits a string, because JSON has no date type;
//! - YAML anchors, tags and merge keys are resolved at load time and not restored;
//! - comments and original formatting are never preserved.

use color_eyre::{eyre::eyre, Result};
use indexmap::IndexMap;
use std::path::Path;

/// A concrete serialisation format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    /// Newline-delimited JSON (also `.ndjson`).
    Jsonl,
    Yaml,
    Toml,
}

impl Format {
    /// Map a file extension (without the dot, any case) to a format.
    pub fn from_ext(ext: &str) -> Option<Format> {
        match ext.to_lowercase().as_str() {
            "json" => Some(Format::Json),
            "jsonl" | "ndjson" | "ldjson" => Some(Format::Jsonl),
            "yaml" | "yml" => Some(Format::Yaml),
            "toml" => Some(Format::Toml),
            _ => None,
        }
    }

    /// Map an explicit `--type` argument to a format.
    pub fn from_name(name: &str) -> Option<Format> {
        Format::from_ext(name)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Jsonl => "jsonl",
            Format::Yaml => "yaml",
            Format::Toml => "toml",
        }
    }
}

/// One node of the document tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    /// RFC-3339 text.  Only TOML produces these on load.
    DateTime(String),
    Arr(Vec<Node>),
    Obj(IndexMap<String, Node>),
}

/// One step of a path into the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seg {
    Key(String),
    Idx(usize),
}

/// Address of a node inside a [`Doc`], from the root.
pub type NodePath = Vec<Seg>;

/// Render a path the way it is shown in breadcrumbs and expanded column names.
pub fn path_to_string(path: &[Seg]) -> String {
    let mut s = String::new();
    for seg in path {
        match seg {
            Seg::Key(k) => {
                if !s.is_empty() {
                    s.push('.');
                }
                s.push_str(k);
            }
            Seg::Idx(i) => s.push_str(&format!("[{}]", i)),
        }
    }
    s
}

/// A parsed document plus the format and path it came from.
pub struct Doc {
    pub format: Format,
    pub root: Node,
    pub path: Option<std::path::PathBuf>,
    /// True when a YAML file held several `---`-separated documents, in which case
    /// `root` is an `Arr` of them and saving back to YAML re-emits the separators.
    pub multi_doc: bool,
}

impl Doc {
    pub fn load(path: &Path, format: Format) -> Result<Doc> {
        let text = std::fs::read_to_string(path)?;
        let mut doc = Doc::from_str(&text, format)?;
        doc.path = Some(path.to_path_buf());
        Ok(doc)
    }

    pub fn from_str(text: &str, format: Format) -> Result<Doc> {
        let (root, multi_doc) = match format {
            Format::Json => (parse_json(text)?, false),
            Format::Jsonl => (parse_jsonl(text)?, false),
            Format::Yaml => parse_yaml(text)?,
            Format::Toml => (parse_toml(text)?, false),
        };
        Ok(Doc {
            format,
            root,
            path: None,
            multi_doc,
        })
    }

    /// Serialise the whole tree into `format`.
    pub fn to_string_as(&self, format: Format, opts: &SaveOpts) -> Result<String> {
        serialize(&self.root, format, self.multi_doc, opts)
    }

    pub fn save_as(&self, path: &Path, format: Format, opts: &SaveOpts) -> Result<()> {
        std::fs::write(path, self.to_string_as(format, opts)?)?;
        Ok(())
    }
}

/// Serialisation knobs, mirroring VisiData's `json_indent` / `json_sort_keys` /
/// `json_ensure_ascii`.
#[derive(Debug, Clone)]
pub struct SaveOpts {
    pub indent: bool,
    pub sort_keys: bool,
}

impl Default for SaveOpts {
    fn default() -> Self {
        SaveOpts {
            indent: true,
            sort_keys: false,
        }
    }
}

// ── tree access ──────────────────────────────────────────────────────────────

impl Node {
    /// Short type name, shown in the `type` column of a key/value view.
    pub fn type_name(&self) -> &'static str {
        match self {
            Node::Null => "null",
            Node::Bool(_) => "bool",
            Node::Int(_) => "int",
            Node::Float(_) => "float",
            Node::Str(_) => "str",
            Node::DateTime(_) => "datetime",
            Node::Arr(_) => "list",
            Node::Obj(_) => "dict",
        }
    }

    pub fn is_container(&self) -> bool {
        matches!(self, Node::Arr(_) | Node::Obj(_))
    }

    pub fn get(&self, path: &[Seg]) -> Option<&Node> {
        let mut cur = self;
        for seg in path {
            cur = match (cur, seg) {
                (Node::Obj(m), Seg::Key(k)) => m.get(k)?,
                (Node::Arr(v), Seg::Idx(i)) => v.get(*i)?,
                _ => return None,
            };
        }
        Some(cur)
    }

    pub fn get_mut(&mut self, path: &[Seg]) -> Option<&mut Node> {
        let mut cur = self;
        for seg in path {
            cur = match (cur, seg) {
                (Node::Obj(m), Seg::Key(k)) => m.get_mut(k)?,
                (Node::Arr(v), Seg::Idx(i)) => v.get_mut(*i)?,
                _ => return None,
            };
        }
        Some(cur)
    }

    /// Replace the node at `path`.  Missing object keys are created; missing array
    /// indices are an error (we never grow arrays implicitly).
    pub fn set(&mut self, path: &[Seg], value: Node) -> Result<()> {
        let Some((last, parents)) = path.split_last() else {
            *self = value;
            return Ok(());
        };
        let parent = self
            .get_mut(parents)
            .ok_or_else(|| eyre!("no node at {}", path_to_string(parents)))?;
        match (parent, last) {
            (Node::Obj(m), Seg::Key(k)) => {
                m.insert(k.clone(), value);
                Ok(())
            }
            (Node::Arr(v), Seg::Idx(i)) => {
                let slot = v
                    .get_mut(*i)
                    .ok_or_else(|| eyre!("index {} out of range", i))?;
                *slot = value;
                Ok(())
            }
            _ => Err(eyre!("cannot set {} on this node", path_to_string(path))),
        }
    }

    /// Compact one-line rendering for a table cell, in the spirit of VisiData's
    /// `iterchars`: `{3} host=localhost port=5432 ssl=true` / `[2] alpha; beta`.
    /// Stops as soon as `max_width` display chars have been produced.
    pub fn render_compact(&self, max_width: usize) -> String {
        let mut out = String::new();
        self.render_into(&mut out, max_width);
        if out.chars().count() > max_width && max_width > 0 {
            out = out.chars().take(max_width.saturating_sub(1)).collect();
            out.push('…');
        }
        out
    }

    fn render_into(&self, out: &mut String, max_width: usize) {
        if out.chars().count() > max_width {
            return;
        }
        match self {
            Node::Null => {}
            Node::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Node::Int(i) => out.push_str(&i.to_string()),
            Node::Float(f) => out.push_str(&fmt_float(*f)),
            Node::Str(s) => out.push_str(s),
            Node::DateTime(s) => out.push_str(s),
            Node::Arr(v) => {
                out.push_str(&format!("[{}]", v.len()));
                for (i, item) in v.iter().enumerate() {
                    if out.chars().count() > max_width {
                        return;
                    }
                    out.push_str(if i == 0 { " " } else { "; " });
                    item.render_into(out, max_width);
                }
            }
            Node::Obj(m) => {
                out.push_str(&format!("{{{}}}", m.len()));
                for (k, v) in m.iter() {
                    if out.chars().count() > max_width {
                        return;
                    }
                    out.push(' ');
                    out.push_str(k);
                    out.push('=');
                    v.render_into(out, max_width);
                }
            }
        }
    }

    /// Plain scalar text, used when a scalar cell is displayed or copied.
    /// Containers fall back to the compact rendering.
    pub fn to_cell_string(&self) -> String {
        match self {
            Node::Null => String::new(),
            Node::Bool(b) => (if *b { "true" } else { "false" }).to_string(),
            Node::Int(i) => i.to_string(),
            Node::Float(f) => fmt_float(*f),
            Node::Str(s) => s.clone(),
            Node::DateTime(s) => s.clone(),
            _ => self.render_compact(usize::MAX),
        }
    }

    /// Parse user-typed text back into a node.
    ///
    /// `old` is the node being replaced; its type is preserved where that is the least
    /// surprising outcome — editing a string cell that happens to read `1.0` must not
    /// silently turn a version number into a float.
    pub fn parse_scalar(text: &str, old: Option<&Node>) -> Node {
        match old {
            Some(Node::Str(_)) => return Node::Str(text.to_string()),
            Some(Node::DateTime(_)) if !text.is_empty() => {
                return Node::DateTime(text.to_string())
            }
            _ => {}
        }
        let t = text.trim();
        if t.is_empty() || t == "null" || t == "~" {
            return Node::Null;
        }
        if t == "true" {
            return Node::Bool(true);
        }
        if t == "false" {
            return Node::Bool(false);
        }
        if let Ok(i) = t.parse::<i64>() {
            return Node::Int(i);
        }
        if let Ok(f) = t.parse::<f64>() {
            return Node::Float(f);
        }
        Node::Str(text.to_string())
    }
}

/// Format a float without a trailing `.0` for whole values, matching how the rest of
/// tuitab renders numbers.
fn fmt_float(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{:.1}", f)
    } else {
        f.to_string()
    }
}

// ── parsing ──────────────────────────────────────────────────────────────────

fn parse_json(text: &str) -> Result<Node> {
    let v: serde_json::Value = serde_json::from_str(text)?;
    Ok(from_json(v))
}

fn parse_jsonl(text: &str) -> Result<Node> {
    let mut rows = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        // blank lines and `//` / `#` comments are skipped, as VisiData does
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| eyre!("line {}: {}", n + 1, e))?;
        rows.push(from_json(v));
    }
    Ok(Node::Arr(rows))
}

fn parse_yaml(text: &str) -> Result<(Node, bool)> {
    use serde::Deserialize;
    let mut docs = Vec::new();
    for de in serde_yaml_ng::Deserializer::from_str(text) {
        let v = serde_yaml_ng::Value::deserialize(de)?;
        if matches!(v, serde_yaml_ng::Value::Null) && docs.is_empty() && text.trim().is_empty() {
            continue;
        }
        docs.push(from_yaml(v));
    }
    match docs.len() {
        0 => Ok((Node::Null, false)),
        1 => Ok((docs.pop().unwrap(), false)),
        _ => Ok((Node::Arr(docs), true)),
    }
}

fn parse_toml(text: &str) -> Result<Node> {
    let v: toml::Value = toml::from_str(text)?;
    Ok(from_toml(v))
}

fn from_json(v: serde_json::Value) -> Node {
    match v {
        serde_json::Value::Null => Node::Null,
        serde_json::Value::Bool(b) => Node::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Node::Int(i)
            } else {
                Node::Float(n.as_f64().unwrap_or(f64::NAN))
            }
        }
        serde_json::Value::String(s) => Node::Str(s),
        serde_json::Value::Array(a) => Node::Arr(a.into_iter().map(from_json).collect()),
        serde_json::Value::Object(o) => {
            Node::Obj(o.into_iter().map(|(k, v)| (k, from_json(v))).collect())
        }
    }
}

fn from_yaml(v: serde_yaml_ng::Value) -> Node {
    use serde_yaml_ng::Value as Y;
    match v {
        Y::Null => Node::Null,
        Y::Bool(b) => Node::Bool(b),
        Y::Number(n) => {
            if let Some(i) = n.as_i64() {
                Node::Int(i)
            } else {
                Node::Float(n.as_f64().unwrap_or(f64::NAN))
            }
        }
        Y::String(s) => Node::Str(s),
        Y::Sequence(s) => Node::Arr(s.into_iter().map(from_yaml).collect()),
        Y::Mapping(m) => Node::Obj(
            m.into_iter()
                .map(|(k, v)| (yaml_key_to_string(k), from_yaml(v)))
                .collect(),
        ),
        // tagged values are flattened to their payload; the tag itself is dropped
        Y::Tagged(t) => from_yaml(t.value),
    }
}

/// YAML permits non-string mapping keys; we render them as text, which is lossy on
/// save (`1:` becomes `"1":`).  Acceptable: non-string keys are rare in config files.
fn yaml_key_to_string(k: serde_yaml_ng::Value) -> String {
    match k {
        serde_yaml_ng::Value::String(s) => s,
        other => match from_yaml(other) {
            Node::Null => String::new(),
            n => n.to_cell_string(),
        },
    }
}

fn from_toml(v: toml::Value) -> Node {
    match v {
        toml::Value::String(s) => Node::Str(s),
        toml::Value::Integer(i) => Node::Int(i),
        toml::Value::Float(f) => Node::Float(f),
        toml::Value::Boolean(b) => Node::Bool(b),
        toml::Value::Datetime(d) => Node::DateTime(d.to_string()),
        toml::Value::Array(a) => Node::Arr(a.into_iter().map(from_toml).collect()),
        toml::Value::Table(t) => Node::Obj(t.into_iter().map(|(k, v)| (k, from_toml(v))).collect()),
    }
}

// ── serialising ──────────────────────────────────────────────────────────────

pub fn serialize(root: &Node, format: Format, multi_doc: bool, opts: &SaveOpts) -> Result<String> {
    let root = if opts.sort_keys {
        &sorted(root)
    } else {
        root
    };
    match format {
        Format::Json => {
            let v = to_json(root);
            Ok(if opts.indent {
                serde_json::to_string_pretty(&v)?
            } else {
                serde_json::to_string(&v)?
            } + "\n")
        }
        Format::Jsonl => {
            let items: Vec<&Node> = match root {
                Node::Arr(v) => v.iter().collect(),
                other => vec![other],
            };
            let mut out = String::new();
            for item in items {
                out.push_str(&serde_json::to_string(&to_json(item))?);
                out.push('\n');
            }
            Ok(out)
        }
        Format::Yaml => {
            if multi_doc {
                if let Node::Arr(docs) = root {
                    let mut out = String::new();
                    for d in docs {
                        out.push_str("---\n");
                        out.push_str(&serde_yaml_ng::to_string(&to_yaml(d))?);
                    }
                    return Ok(out);
                }
            }
            Ok(serde_yaml_ng::to_string(&to_yaml(root))?)
        }
        Format::Toml => {
            let v = to_toml(root)
                .ok_or_else(|| eyre!("TOML cannot represent a null document"))?;
            if !matches!(v, toml::Value::Table(_)) {
                return Err(eyre!(
                    "TOML requires a table at the top level, got {}",
                    root.type_name()
                ));
            }
            Ok(if opts.indent {
                toml::to_string_pretty(&v)?
            } else {
                toml::to_string(&v)?
            })
        }
    }
}

fn sorted(n: &Node) -> Node {
    match n {
        Node::Obj(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            Node::Obj(keys.into_iter().map(|k| (k.clone(), sorted(&m[k]))).collect())
        }
        Node::Arr(v) => Node::Arr(v.iter().map(sorted).collect()),
        other => other.clone(),
    }
}

fn to_json(n: &Node) -> serde_json::Value {
    use serde_json::Value as J;
    match n {
        Node::Null => J::Null,
        Node::Bool(b) => J::Bool(*b),
        Node::Int(i) => J::Number((*i).into()),
        Node::Float(f) => serde_json::Number::from_f64(*f).map(J::Number).unwrap_or(J::Null),
        // JSON has no date type, so a datetime degrades to its RFC-3339 text
        Node::Str(s) | Node::DateTime(s) => J::String(s.clone()),
        Node::Arr(v) => J::Array(v.iter().map(to_json).collect()),
        Node::Obj(m) => J::Object(m.iter().map(|(k, v)| (k.clone(), to_json(v))).collect()),
    }
}

fn to_yaml(n: &Node) -> serde_yaml_ng::Value {
    use serde_yaml_ng::Value as Y;
    match n {
        Node::Null => Y::Null,
        Node::Bool(b) => Y::Bool(*b),
        Node::Int(i) => Y::Number((*i).into()),
        Node::Float(f) => Y::Number((*f).into()),
        Node::Str(s) | Node::DateTime(s) => Y::String(s.clone()),
        Node::Arr(v) => Y::Sequence(v.iter().map(to_yaml).collect()),
        Node::Obj(m) => Y::Mapping(
            m.iter()
                .map(|(k, v)| (Y::String(k.clone()), to_yaml(v)))
                .collect(),
        ),
    }
}

/// TOML has no null.  A null value returns `None`, and callers drop the key (objects)
/// or the element (arrays) — the same choice VisiData makes with `keep_nulls=False`.
fn to_toml(n: &Node) -> Option<toml::Value> {
    Some(match n {
        Node::Null => return None,
        Node::Bool(b) => toml::Value::Boolean(*b),
        Node::Int(i) => toml::Value::Integer(*i),
        Node::Float(f) => toml::Value::Float(*f),
        Node::Str(s) => toml::Value::String(s.clone()),
        Node::DateTime(s) => match s.parse::<toml::value::Datetime>() {
            Ok(d) => toml::Value::Datetime(d),
            Err(_) => toml::Value::String(s.clone()),
        },
        Node::Arr(v) => toml::Value::Array(v.iter().filter_map(to_toml).collect()),
        Node::Obj(m) => toml::Value::Table(
            m.iter()
                .filter_map(|(k, v)| to_toml(v).map(|v| (k.clone(), v)))
                .collect(),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(pairs: &[(&str, Node)]) -> Node {
        Node::Obj(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        )
    }

    #[test]
    fn json_roundtrip_preserves_key_order() {
        let src = r#"{"zebra": 1, "apple": 2, "mango": [1, 2, 3]}"#;
        let doc = Doc::from_str(src, Format::Json).unwrap();
        let out = doc.to_string_as(Format::Json, &SaveOpts::default()).unwrap();
        let keys: Vec<&str> = out
            .lines()
            .filter_map(|l| l.trim().split('"').nth(1))
            .collect();
        assert_eq!(keys[0], "zebra", "key order must survive: {}", out);
        assert_eq!(Doc::from_str(&out, Format::Json).unwrap().root, doc.root);
    }

    #[test]
    fn toml_datetime_survives_roundtrip() {
        let src = "released = 1979-05-27T07:32:00Z\nname = \"x\"\n";
        let doc = Doc::from_str(src, Format::Toml).unwrap();
        assert!(matches!(
            doc.root.get(&[Seg::Key("released".into())]),
            Some(Node::DateTime(_))
        ));
        let out = doc.to_string_as(Format::Toml, &SaveOpts::default()).unwrap();
        assert!(
            out.contains("1979-05-27T07:32:00Z"),
            "datetime must not become a quoted string: {}",
            out
        );
        // and converting to JSON degrades it to text rather than corrupting it
        let js = doc.to_string_as(Format::Json, &SaveOpts::default()).unwrap();
        assert!(js.contains("\"1979-05-27T07:32:00Z\""), "{}", js);
    }

    #[test]
    fn yaml_multidoc_becomes_array_and_comes_back() {
        let src = "a: 1\n---\na: 2\n";
        let doc = Doc::from_str(src, Format::Yaml).unwrap();
        assert!(doc.multi_doc);
        assert_eq!(doc.root, Node::Arr(vec![
            obj(&[("a", Node::Int(1))]),
            obj(&[("a", Node::Int(2))]),
        ]));
        let out = doc.to_string_as(Format::Yaml, &SaveOpts::default()).unwrap();
        assert_eq!(out.matches("---").count(), 2, "{}", out);
        assert_eq!(Doc::from_str(&out, Format::Yaml).unwrap().root, doc.root);
    }

    #[test]
    fn jsonl_skips_blanks_and_comments() {
        let src = "// header\n{\"a\":1}\n\n{\"a\":2}\n";
        let doc = Doc::from_str(src, Format::Jsonl).unwrap();
        assert_eq!(
            doc.root,
            Node::Arr(vec![obj(&[("a", Node::Int(1))]), obj(&[("a", Node::Int(2))])])
        );
        let out = doc.to_string_as(Format::Jsonl, &SaveOpts::default()).unwrap();
        assert_eq!(out, "{\"a\":1}\n{\"a\":2}\n");
    }

    #[test]
    fn toml_drops_nulls_and_rejects_non_table_root() {
        let root = obj(&[("keep", Node::Int(1)), ("drop", Node::Null)]);
        let doc = Doc {
            format: Format::Toml,
            root,
            path: None,
            multi_doc: false,
        };
        let out = doc.to_string_as(Format::Toml, &SaveOpts::default()).unwrap();
        assert!(out.contains("keep"), "{}", out);
        assert!(!out.contains("drop"), "null key must be dropped: {}", out);

        let arr = Doc {
            format: Format::Toml,
            root: Node::Arr(vec![Node::Int(1)]),
            path: None,
            multi_doc: false,
        };
        assert!(arr.to_string_as(Format::Toml, &SaveOpts::default()).is_err());
    }

    #[test]
    fn set_and_get_by_path() {
        let mut root = obj(&[(
            "servers",
            Node::Arr(vec![obj(&[("host", Node::Str("a".into()))])]),
        )]);
        let p = vec![Seg::Key("servers".into()), Seg::Idx(0), Seg::Key("host".into())];
        assert_eq!(root.get(&p), Some(&Node::Str("a".into())));
        root.set(&p, Node::Str("b".into())).unwrap();
        assert_eq!(root.get(&p), Some(&Node::Str("b".into())));
        assert_eq!(path_to_string(&p), "servers[0].host");

        // growing an array implicitly is refused rather than silently ignored
        let bad = vec![Seg::Key("servers".into()), Seg::Idx(9)];
        assert!(root.set(&bad, Node::Null).is_err());
    }

    #[test]
    fn parse_scalar_keeps_string_typed_cells_as_strings() {
        let old = Node::Str("1.0".into());
        assert_eq!(Node::parse_scalar("2.0", Some(&old)), Node::Str("2.0".into()));
        assert_eq!(Node::parse_scalar("2.0", None), Node::Float(2.0));
        assert_eq!(Node::parse_scalar("7", None), Node::Int(7));
        assert_eq!(Node::parse_scalar("", None), Node::Null);
        assert_eq!(Node::parse_scalar("true", None), Node::Bool(true));
    }

    #[test]
    fn render_compact_matches_visidata_shape_and_clips() {
        let n = obj(&[("host", Node::Str("localhost".into())), ("port", Node::Int(5432))]);
        assert_eq!(n.render_compact(100), "{2} host=localhost port=5432");
        assert_eq!(
            Node::Arr(vec![Node::Str("alpha".into()), Node::Str("beta".into())]).render_compact(100),
            "[2] alpha; beta"
        );
        let clipped = n.render_compact(10);
        assert_eq!(clipped.chars().count(), 10, "{:?}", clipped);
        assert!(clipped.ends_with('…'));
    }
}
