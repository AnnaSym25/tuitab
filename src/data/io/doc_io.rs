//! Wiring between the document tree and the rest of the app.
//!
//! [`DocState`] is what a sheet carries when its data came from JSON/JSONL/YAML/TOML:
//! a shared tree, the [`View`] projecting part of it, and the row/column bookkeeping
//! needed to turn a table cell back into a [`NodePath`].  Every sheet in a dive chain
//! shares the same tree, so an edit made three levels down is visible when you pop back.

use crate::data::dataframe::DataFrame;
use crate::data::doc::{Doc, Format, Node, NodePath, SaveOpts, Seg};
use crate::data::view::{cell_path, ColRole, View, ViewMode};
use crate::types::ColumnType;
use color_eyre::{eyre::eyre, Result};
use indexmap::IndexMap;
use polars::prelude::AnyValue;
use std::path::Path;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

pub struct DocState {
    pub doc: Arc<RwLock<Doc>>,
    pub view: View,
    pub row_paths: Vec<NodePath>,
    pub col_roles: Vec<ColRole>,
}

impl DocState {
    pub fn open(path: &Path, format: Format) -> Result<(DataFrame, DocState)> {
        DocState::from_doc(Doc::load(path, format)?)
    }

    pub fn from_doc(doc: Doc) -> Result<(DataFrame, DocState)> {
        let view = View::auto(vec![], &doc.root);
        DocState::build(Arc::new(RwLock::new(doc)), view)
    }

    fn build(doc: Arc<RwLock<Doc>>, view: View) -> Result<(DataFrame, DocState)> {
        let proj = {
            let guard = doc.read().map_err(|_| eyre!("document lock poisoned"))?;
            view.project(&guard.root)?
        };
        Ok((
            proj.df,
            DocState {
                doc,
                view,
                row_paths: proj.row_paths,
                col_roles: proj.col_roles,
            },
        ))
    }

    /// Build a sheet anchored anywhere in a document you already hold a handle to.
    /// Used to jump straight to a search hit, where there is no parent sheet to dive
    /// from — only the tree and a path.
    pub fn open_at(
        doc: Arc<RwLock<Doc>>,
        anchor: NodePath,
    ) -> Result<(DataFrame, DocState)> {
        let view = {
            let guard = doc.read().map_err(|_| eyre!("document lock poisoned"))?;
            let node = guard
                .root
                .get(&anchor)
                .ok_or_else(|| eyre!("that node is gone"))?;
            View::auto(anchor, node)
        };
        DocState::build(doc, view)
    }

    /// Open a child sheet anchored at `anchor`, sharing this document.
    pub fn dive(&self, anchor: NodePath) -> Result<(DataFrame, DocState)> {
        let view = {
            let guard = self.doc.read().map_err(|_| eyre!("document lock poisoned"))?;
            let node = guard
                .root
                .get(&anchor)
                .ok_or_else(|| eyre!("nothing to dive into"))?;
            if !node.is_container() {
                return Err(eyre!("{} is not a container", node.type_name()));
            }
            View::auto(anchor, node)
        };
        DocState::build(Arc::clone(&self.doc), view)
    }

    /// Re-run the projection after the tree changed shape.  Used when a whole node was
    /// replaced or the view mode changed — a plain cell edit patches in place instead.
    pub fn reproject(&mut self) -> Result<DataFrame> {
        let proj = {
            let guard = self.doc.read().map_err(|_| eyre!("document lock poisoned"))?;
            self.view.project(&guard.root)?
        };
        self.row_paths = proj.row_paths;
        self.col_roles = proj.col_roles;
        Ok(proj.df)
    }

    pub fn set_mode(&mut self, mode: ViewMode) -> Result<DataFrame> {
        self.view.mode = mode;
        self.view.expanded.clear();
        self.reproject()
    }

    /// Replace the column with one column per child of its containers (`(`).
    pub fn expand_column(&mut self, col: usize) -> Result<DataFrame> {
        // Only record columns expand.  In key/value mode every row is a different node,
        // so there is no common set of children to make columns out of — diving is the
        // right move there, and the message has to say so rather than claim the column
        // holds no containers when it plainly does.
        let ColRole::Field(path) = self.column_role(col)? else {
            return Err(eyre!(
                "expand works on record columns — press Enter to dive in here"
            ));
        };
        if !self.column_has_container(col) {
            return Err(eyre!("nothing to expand — this column holds no containers"));
        }
        if !self.view.expand(path) {
            return Err(eyre!("already expanded"));
        }
        self.reproject()
    }

    /// Delete the given physical rows from the document.
    ///
    /// Unlike a column, a row *is* data: in records mode it is an array element, in
    /// key/value mode a key of the object.  Hiding it from the table without touching
    /// the tree — which is what the plain DataFrame path does — would look like a
    /// deletion and then quietly undo itself on the next save.
    ///
    /// Array indices are removed high-to-low so the earlier ones stay valid.
    pub fn delete_rows(&mut self, rows: &HashSet<usize>) -> Result<DataFrame> {
        let mut paths: Vec<NodePath> = rows
            .iter()
            .filter_map(|r| self.row_paths.get(*r).cloned())
            .collect();
        if paths.is_empty() {
            return Err(eyre!("nothing to delete"));
        }
        paths.sort_by(|a, b| match (a.last(), b.last()) {
            (Some(Seg::Idx(x)), Some(Seg::Idx(y))) => y.cmp(x),
            _ => std::cmp::Ordering::Equal,
        });
        {
            let mut guard = self.doc.write().map_err(|_| eyre!("document lock poisoned"))?;
            for path in &paths {
                guard.root.remove(path)?;
            }
            guard.bump();
        }
        self.reproject()
    }

    /// Fold the innermost expansion covering this column back into one column (`)`).
    pub fn contract_column(&mut self, col: usize) -> Result<DataFrame> {
        let ColRole::Field(path) = self.column_role(col)? else {
            return Err(eyre!("nothing to contract here"));
        };
        if !self.view.contract_one(&path) {
            return Err(eyre!("this column is not expanded"));
        }
        self.reproject()
    }

    fn column_role(&self, col: usize) -> Result<ColRole> {
        self.col_roles
            .get(col)
            .cloned()
            .ok_or_else(|| eyre!("no such column"))
    }

    fn column_has_container(&self, col: usize) -> bool {
        (0..self.row_paths.len())
            .filter_map(|r| self.node_at(r, col))
            .any(|n| n.is_container())
    }

    /// Modes offered by `m` for the currently anchored node.
    pub fn available_modes(&self) -> Vec<ViewMode> {
        let Ok(guard) = self.doc.read() else {
            return vec![self.view.mode];
        };
        match guard.root.get(&self.view.anchor) {
            Some(node) => View::modes_for(node),
            None => vec![self.view.mode],
        }
    }

    pub fn path_of(&self, row: usize, col: usize) -> Option<NodePath> {
        cell_path(&self.row_paths, &self.col_roles, row, col)
    }

    pub fn node_at(&self, row: usize, col: usize) -> Option<Node> {
        let path = self.path_of(row, col)?;
        let guard = self.doc.read().ok()?;
        guard.root.get(&path).cloned()
    }

    /// Write an edited scalar into the tree.  Returns the text the table cell should
    /// now show, so the caller can patch the DataFrame without a full reprojection
    /// (which would drop the user's sort and selection).
    pub fn set_cell(&mut self, row: usize, col: usize, text: &str) -> Result<String> {
        let role = self
            .col_roles
            .get(col)
            .ok_or_else(|| eyre!("no such column"))?
            .clone();
        if role == ColRole::Type {
            return Err(eyre!("the type column is read-only"));
        }
        let path = self
            .path_of(row, col)
            .ok_or_else(|| eyre!("this cell does not address a node"))?;

        let mut guard = self.doc.write().map_err(|_| eyre!("document lock poisoned"))?;

        if role == ColRole::Key {
            let out = rename_key(&mut guard.root, &path, text).map(|_| text.to_string());
            if out.is_ok() {
                guard.bump();
            }
            return out;
        }

        let old = guard.root.get(&path).cloned();
        if matches!(old, Some(ref n) if n.is_container()) {
            return Err(eyre!("press E to edit this container in $EDITOR"));
        }
        let new = Node::parse_scalar(text, old.as_ref());
        let shown = new.to_cell_string();
        guard.root.set(&path, new)?;
        guard.bump();
        Ok(shown)
    }

    /// Serialise the node under the cursor in the document's own format, for the
    /// "edit node as text" popup.
    pub fn node_text(&self, path: &[Seg]) -> Result<String> {
        let guard = self.doc.read().map_err(|_| eyre!("document lock poisoned"))?;
        let node = guard
            .root
            .get(path)
            .ok_or_else(|| eyre!("no node at that path"))?;
        let fmt = text_edit_format(guard.format, node);
        crate::data::doc::serialize(node, fmt, false, &SaveOpts::default())
    }

    /// Parse the text from the "edit node as text" popup and replace the node.
    /// On a parse error nothing is written and the caller keeps the popup open.
    pub fn set_node_text(&mut self, path: &[Seg], text: &str) -> Result<()> {
        let fmt = {
            let guard = self.doc.read().map_err(|_| eyre!("document lock poisoned"))?;
            let node = guard
                .root
                .get(path)
                .ok_or_else(|| eyre!("no node at that path"))?;
            text_edit_format(guard.format, node)
        };
        let parsed = Doc::from_str(text, fmt)?.root;
        let mut guard = self.doc.write().map_err(|_| eyre!("document lock poisoned"))?;
        guard.root.set(path, parsed)?;
        guard.bump();
        Ok(())
    }

    pub fn format(&self) -> Format {
        self.doc.read().map(|d| d.format).unwrap_or(Format::Json)
    }

    pub fn save(&self, path: &Path, format: Format, opts: &SaveOpts) -> Result<()> {
        let guard = self.doc.read().map_err(|_| eyre!("document lock poisoned"))?;
        guard.save_as(path, format, opts)
    }

    /// Save, wrapping the document if the target format cannot hold its shape.
    ///
    /// Only TOML needs this: its top level must be a table, so a JSON array of records
    /// becomes an array of tables named after the sheet.  Without the wrap, converting
    /// `data.json` to `data.toml` would just fail with nothing the user could do about
    /// it, and conversion is the whole point of writing through the tree.
    pub fn save_wrapped(
        &self,
        path: &Path,
        format: Format,
        opts: &SaveOpts,
        name: &str,
    ) -> Result<()> {
        let guard = self.doc.read().map_err(|_| eyre!("document lock poisoned"))?;
        if format == Format::Toml && !matches!(guard.root, Node::Obj(_)) {
            let wrapped = Doc {
                format,
                source_text: None,
                root: Node::Obj(
                    [(toml_table_name(name), guard.root.clone())]
                        .into_iter()
                        .collect::<IndexMap<_, _>>(),
                ),
                path: None,
                multi_doc: false,
                revision: 0,
            };
            return wrapped.save_as(path, format, opts);
        }
        guard.save_as(path, format, opts)
    }

    /// What this document loses if written as `target`, if anything.
    ///
    /// Saving is not the moment to discover that a conversion was one-way, so the
    /// caller shows this before the write rather than after.
    pub fn conversion_loss(&self, target: Format) -> Option<String> {
        let guard = self.doc.read().ok()?;
        let mut lost: Vec<&str> = Vec::new();
        if guard.multi_doc && target != Format::Yaml {
            lost.push("document separators");
        }
        // JSONL has no way to say "this is one document rather than a stream", so a
        // document that is not already a list reads back as a one-record stream.  The
        // data survives exactly; the shape does not.
        if target == Format::Jsonl && !matches!(guard.root, Node::Arr(_)) {
            lost.push("the difference between a document and a one-record stream");
        }
        if guard.format == Format::Toml
            && target != Format::Toml
            && guard
                .source_text
                .as_deref()
                .is_some_and(|s| s.lines().any(|l| l.trim_start().starts_with('#')))
        {
            lost.push("comments");
        }
        if lost.is_empty() {
            None
        } else {
            Some(format!(
                "{} cannot carry {}",
                target.name().to_uppercase(),
                lost.join(" or ")
            ))
        }
    }

    /// Current revision of the shared document.
    pub fn revision(&self) -> u64 {
        self.doc.read().map(|d| d.revision).unwrap_or_default()
    }

    /// Breadcrumb trail shown in the sheet title: `config.toml › servers › [1]`.
    pub fn breadcrumbs(&self, root_name: &str) -> String {
        let mut s = String::from(root_name);
        for seg in &self.view.anchor {
            s.push_str(" › ");
            match seg {
                Seg::Key(k) => s.push_str(k),
                Seg::Idx(i) => s.push_str(&format!("[{}]", i)),
            }
        }
        s
    }
}

/// A TOML fragment is usually not a valid TOML document on its own (an array, a scalar,
/// a nested table with no name), so node-level text editing of a TOML file goes through
/// JSON.  Whole tables stay in TOML, which is what a user editing a config expects.
fn text_edit_format(doc_format: Format, node: &Node) -> Format {
    match doc_format {
        Format::Toml if !matches!(node, Node::Obj(_)) => Format::Json,
        Format::Jsonl => Format::Json,
        other => other,
    }
}

/// Rename an object key in place, keeping its position in the key order.
fn rename_key(root: &mut Node, path: &[Seg], new_key: &str) -> Result<()> {
    let Some((Seg::Key(old_key), parents)) = path.split_last().map(|(l, p)| (l.clone(), p)) else {
        return Err(eyre!("only object keys can be renamed"));
    };
    if new_key.is_empty() {
        return Err(eyre!("key cannot be empty"));
    }
    let parent = root
        .get_mut(parents)
        .ok_or_else(|| eyre!("parent node is gone"))?;
    let Node::Obj(map) = parent else {
        return Err(eyre!("parent is not an object"));
    };
    if old_key == new_key {
        return Ok(());
    }
    if map.contains_key(new_key) {
        return Err(eyre!("key `{}` already exists", new_key));
    }
    let idx = map
        .get_index_of(old_key.as_str())
        .ok_or_else(|| eyre!("key `{}` is gone", old_key))?;
    let (_, value) = map.shift_remove_index(idx).expect("index just looked up");
    map.shift_insert(idx, new_key.to_string(), value);
    Ok(())
}

// ── table → document ─────────────────────────────────────────────────────────

/// How a plain table (CSV, Parquet, SQL, pivot — anything with no document behind it)
/// is turned into a tree before being written as JSON/YAML/TOML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shape {
    /// `[{col: val}, …]` — the default, and the only shape that survives a round trip.
    #[default]
    Records,
    /// `{col: [val, …]}`
    Columns,
    /// `{a: b}` — only for a two-column table.
    KeyValue,
}

impl Shape {
    pub fn label(&self) -> &'static str {
        match self {
            Shape::Records => "records",
            Shape::Columns => "columns",
            Shape::KeyValue => "key/value",
        }
    }

    /// One-line example of what this shape produces, shown next to the label.
    pub fn hint(&self) -> &'static str {
        match self {
            Shape::Records => "[{col: val}, …]  — round-trips",
            Shape::Columns => "{col: [val, …]}",
            Shape::KeyValue => "{a: b} — first column becomes the key",
        }
    }

    /// Shapes that make sense for a table of `ncols` columns, best first.
    pub fn options(ncols: usize) -> Vec<Shape> {
        let mut v = vec![Shape::Records, Shape::Columns];
        if ncols == 2 {
            v.push(Shape::KeyValue);
        }
        v
    }
}


/// Build a document from a table.
///
/// `table_name` is only used for TOML, whose top level must be a table: records become
/// an array of tables under that name.
pub fn table_to_doc(
    df: &DataFrame,
    shape: Shape,
    format: Format,
    table_name: &str,
) -> Result<Doc> {
    let ncols = df.col_count();
    let nrows = df.visible_row_count();
    if ncols == 0 {
        return Err(eyre!("nothing to save: the sheet has no columns"));
    }
    let names: Vec<String> = df.columns.iter().map(|c| c.name.clone()).collect();

    let root = match shape {
        Shape::Records => {
            let rows: Vec<Node> = (0..nrows)
                .map(|r| {
                    let mut obj = IndexMap::new();
                    for (c, name) in names.iter().enumerate() {
                        let v = cell_node(df, r, c);
                        // Every column is kept on the first row so the output carries a
                        // full header even when later rows are sparse — the same reason
                        // VisiData passes keep_nulls only for row 0.
                        if r == 0 || !matches!(v, Node::Null) {
                            obj.insert(name.clone(), v);
                        }
                    }
                    Node::Obj(obj)
                })
                .collect();
            let arr = Node::Arr(rows);
            if format == Format::Toml {
                Node::Obj(
                    [(toml_table_name(table_name), arr)]
                        .into_iter()
                        .collect::<IndexMap<_, _>>(),
                )
            } else {
                arr
            }
        }
        Shape::Columns => Node::Obj(
            (0..ncols)
                .map(|c| {
                    let col: Vec<Node> = (0..nrows).map(|r| cell_node(df, r, c)).collect();
                    (names[c].clone(), Node::Arr(col))
                })
                .collect(),
        ),
        Shape::KeyValue => {
            if ncols != 2 {
                return Err(eyre!(
                    "key/value shape needs exactly 2 columns, this sheet has {}",
                    ncols
                ));
            }
            let mut obj = IndexMap::new();
            for r in 0..nrows {
                let key = match cell_node(df, r, 0) {
                    Node::Null => continue,
                    k => k.to_cell_string(),
                };
                obj.insert(key, cell_node(df, r, 1));
            }
            Node::Obj(obj)
        }
    };

    Ok(Doc {
        format,
        root,
        path: None,
        source_text: None,
        multi_doc: false,
        revision: 0,
    })
}

/// TOML array-of-tables needs a name; fall back to `rows` when the sheet title yields
/// nothing usable.  Keys are quoted by the serialiser if they are not bare-legal, so no
/// sanitising is needed beyond dropping whitespace and any extension the sheet title
/// carries (a sheet is usually named after its file).
fn toml_table_name(name: &str) -> String {
    let stem = name.rsplit('/').next().unwrap_or(name);
    let stem = stem.split('.').next().unwrap_or(stem);
    let cleaned: String = stem
        .trim()
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect();
    if cleaned.is_empty() {
        "rows".to_string()
    } else {
        cleaned
    }
}

fn cell_node(df: &DataFrame, display_row: usize, col: usize) -> Node {
    match df.get_val(display_row, col) {
        AnyValue::Null => Node::Null,
        AnyValue::Boolean(b) => Node::Bool(b),
        AnyValue::Int8(i) => Node::Int(i as i64),
        AnyValue::Int16(i) => Node::Int(i as i64),
        AnyValue::Int32(i) => Node::Int(i as i64),
        AnyValue::Int64(i) => Node::Int(i),
        AnyValue::UInt8(i) => Node::Int(i as i64),
        AnyValue::UInt16(i) => Node::Int(i as i64),
        AnyValue::UInt32(i) => Node::Int(i as i64),
        AnyValue::UInt64(i) => Node::Int(i as i64),
        AnyValue::Float32(f) => Node::Float(f as f64),
        AnyValue::Float64(f) => Node::Float(f),
        AnyValue::String(s) => scalar_from_text(df, col, s),
        AnyValue::StringOwned(s) => scalar_from_text(df, col, s.as_str()),
        other => {
            let s = other.to_string();
            if s.is_empty() {
                Node::Null
            } else {
                Node::Str(s)
            }
        }
    }
}

/// A string cell in a column typed as a date keeps its text; everything else stays a
/// string.  We deliberately do not sniff numbers out of text columns — a zip code must
/// not become an integer on export.
fn scalar_from_text(df: &DataFrame, col: usize, s: &str) -> Node {
    if s.is_empty() {
        return Node::Null;
    }
    match df.columns.get(col).map(|c| c.col_type) {
        Some(ColumnType::Date) | Some(ColumnType::Datetime) => Node::DateTime(s.to_string()),
        _ => Node::Str(s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::doc::Format;

    fn state(src: &str, f: Format) -> (DataFrame, DocState) {
        DocState::from_doc(Doc::from_str(src, f).unwrap()).unwrap()
    }

    #[test]
    fn editing_a_cell_writes_through_to_the_tree() {
        let (df, mut st) = state(r#"[{"a":1},{"a":2}]"#, Format::Json);
        assert_eq!(df.get_physical(0, 0), "1");
        assert_eq!(st.set_cell(0, 0, "42").unwrap(), "42");
        let out = st
            .doc
            .read()
            .unwrap()
            .to_string_as(Format::Json, &SaveOpts { indent: false, sort_keys: false })
            .unwrap();
        assert_eq!(out.trim(), r#"[{"a":42},{"a":2}]"#);
    }

    #[test]
    fn string_cells_stay_strings_when_edited() {
        let (_df, mut st) = state(r#"[{"v":"1.0"}]"#, Format::Json);
        st.set_cell(0, 0, "2.0").unwrap();
        let out = st
            .doc
            .read()
            .unwrap()
            .to_string_as(Format::Json, &SaveOpts { indent: false, sort_keys: false })
            .unwrap();
        assert_eq!(out.trim(), r#"[{"v":"2.0"}]"#, "must not become a number");
    }

    #[test]
    fn container_cells_refuse_inline_edits() {
        let (_df, mut st) = state(r#"{"db":{"host":"h"}}"#, Format::Json);
        let err = st.set_cell(0, 1, "nonsense").unwrap_err().to_string();
        assert!(err.contains("E to edit"), "{}", err);
    }

    #[test]
    fn key_column_renames_and_keeps_position() {
        let (_df, mut st) = state("a = 1\nb = 2\nc = 3\n", Format::Toml);
        st.set_cell(1, 0, "bb").unwrap();
        let out = st
            .doc
            .read()
            .unwrap()
            .to_string_as(Format::Toml, &SaveOpts::default())
            .unwrap();
        let keys: Vec<&str> = out
            .lines()
            .filter_map(|l| l.split(" =").next())
            .filter(|l| !l.is_empty())
            .collect();
        assert_eq!(keys, vec!["a", "bb", "c"], "{}", out);
    }

    #[test]
    fn renaming_onto_an_existing_key_is_refused() {
        let (_df, mut st) = state("a = 1\nb = 2\n", Format::Toml);
        assert!(st.set_cell(1, 0, "a").is_err());
    }

    #[test]
    fn diving_shares_the_tree_so_edits_are_visible_from_the_parent() {
        let (_df, parent) = state(r#"{"servers":[{"host":"a"}]}"#, Format::Json);
        let (child_df, mut child) = parent.dive(vec![Seg::Key("servers".into())]).unwrap();
        assert_eq!(child_df.get_physical(0, 0), "a");
        child.set_cell(0, 0, "b").unwrap();
        let out = parent
            .doc
            .read()
            .unwrap()
            .to_string_as(Format::Json, &SaveOpts { indent: false, sort_keys: false })
            .unwrap();
        assert_eq!(out.trim(), r#"{"servers":[{"host":"b"}]}"#);
    }

    #[test]
    fn node_text_edit_replaces_a_whole_subtree() {
        let (_df, mut st) = state(r#"{"db":{"host":"h"}}"#, Format::Json);
        let path = vec![Seg::Key("db".into())];
        assert_eq!(st.node_text(&path).unwrap().trim(), "{\n  \"host\": \"h\"\n}");
        st.set_node_text(&path, r#"{"host":"h2","port":5432}"#).unwrap();
        let out = st
            .doc
            .read()
            .unwrap()
            .to_string_as(Format::Json, &SaveOpts { indent: false, sort_keys: false })
            .unwrap();
        assert_eq!(out.trim(), r#"{"db":{"host":"h2","port":5432}}"#);
    }

    #[test]
    fn a_bad_node_text_edit_changes_nothing() {
        let (_df, mut st) = state(r#"{"db":{"host":"h"}}"#, Format::Json);
        let path = vec![Seg::Key("db".into())];
        assert!(st.set_node_text(&path, "{not json").is_err());
        let out = st
            .doc
            .read()
            .unwrap()
            .to_string_as(Format::Json, &SaveOpts { indent: false, sort_keys: false })
            .unwrap();
        assert_eq!(out.trim(), r#"{"db":{"host":"h"}}"#);
    }

    #[test]
    fn switching_view_mode_reprojects() {
        let (df, mut st) = state(r#"{"a":1,"b":2}"#, Format::Json);
        assert_eq!(df.df.width(), 3, "key/value by default");
        let df = st.set_mode(ViewMode::Records).unwrap();
        assert_eq!(df.df.width(), 2, "records shows one column per key");
        assert_eq!(df.df.height(), 1);
    }

    #[test]
    fn table_to_toml_records_becomes_an_array_of_tables() {
        let (df, _) = state(r#"[{"a":1},{"a":2}]"#, Format::Json);
        let doc = table_to_doc(&df, Shape::Records, Format::Toml, "my sheet").unwrap();
        let out = doc.to_string_as(Format::Toml, &SaveOpts::default()).unwrap();
        assert!(out.contains("[[my_sheet]]"), "{}", out);
        assert_eq!(out.matches("[[my_sheet]]").count(), 2, "{}", out);
    }

    #[test]
    fn table_to_json_keeps_the_full_header_on_the_first_row_only() {
        let (df, _) = state(r#"[{"a":1,"b":null},{"a":2,"b":null}]"#, Format::Json);
        let doc = table_to_doc(&df, Shape::Records, Format::Json, "x").unwrap();
        let out = doc
            .to_string_as(Format::Json, &SaveOpts { indent: false, sort_keys: false })
            .unwrap();
        assert_eq!(out.trim(), r#"[{"a":1,"b":null},{"a":2}]"#);
    }

    #[test]
    fn key_value_shape_needs_two_columns() {
        let (df, _) = state(r#"[{"a":1,"b":2,"c":3}]"#, Format::Json);
        assert!(table_to_doc(&df, Shape::KeyValue, Format::Json, "x").is_err());
    }
}
