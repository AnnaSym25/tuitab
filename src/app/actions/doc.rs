//! Actions for sheets backed by a document tree (JSON/JSONL/YAML/TOML): diving into
//! nodes and switching how the anchored node is projected.
//!
//! Editing lives elsewhere: scalar cells go through [`super::edit`], and whole nodes go
//! through the existing `$EDITOR` integration in [`crate::app`], which is doc-aware.

use crate::app::App;
use crate::data::doc::NodePath;

impl App {
    /// Physical row index under the cursor, or `None` when the sheet is empty.
    pub(crate) fn cursor_physical_row(&self) -> Option<usize> {
        let s = self.stack.active();
        let display_row = s.table_state.selected()?;
        if display_row >= s.dataframe.visible_row_count() {
            return None;
        }
        s.dataframe.row_order.get(display_row).copied()
    }

    /// Path of the node the cursor is on: the row's node, or the cell's node when
    /// `cell` is set (`zEnter` vs `Enter`).
    pub(crate) fn cursor_node_path(&self, cell: bool) -> Option<NodePath> {
        let s = self.stack.active();
        let doc = s.doc.as_ref()?;
        let row = self.cursor_physical_row()?;
        if cell {
            doc.path_of(row, s.cursor_col)
        } else {
            doc.row_paths.get(row).cloned()
        }
    }

    /// Push a sheet anchored at the node under the cursor.  Scalars have nothing to
    /// dive into and say so rather than pushing an empty sheet.
    pub(crate) fn dive_into_node(&mut self, cell: bool) {
        let Some(path) = self.cursor_node_path(cell) else {
            self.status_message = "No node to dive into".to_string();
            return;
        };
        let s = self.stack.active();
        let Some(doc) = s.doc.as_ref() else { return };

        match doc.dive(path) {
            Ok((df, state)) => {
                let title = state.breadcrumbs(&root_name(&s.title));
                let rows = df.visible_row_count();
                let source = s.source_path.clone();
                let mut sheet = crate::sheet::Sheet::new(title, df);
                sheet.doc = Some(state);
                sheet.source_path = source;
                self.stack.push(sheet);
                self.status_message = format!("{} rows", rows);
            }
            Err(e) => self.status_message = e.to_string(),
        }
    }

    /// Rebuild a doc-backed sheet's table from the shared tree.
    ///
    /// Called after popping back from a dive: the child may have edited a node this
    /// sheet renders, and the table is only a cached projection.  The cursor is kept
    /// where it was, since the row count rarely changes and losing the position on
    /// every `Esc` would be worse than the occasional off-by-one.
    pub(crate) fn refresh_doc_projection(&mut self) {
        let s = self.stack.active_mut();
        if s.doc.is_none() {
            return;
        }
        let row = s.table_state.selected();
        let col = s.cursor_col;
        let doc = s.doc.as_mut().expect("checked above");
        if let Ok(df) = doc.reproject() {
            s.dataframe = df;
            let rows = s.dataframe.visible_row_count();
            let cols = s.dataframe.columns.len();
            s.table_state
                .select(row.map(|r| r.min(rows.saturating_sub(1))));
            s.cursor_col = col.min(cols.saturating_sub(1));
            s.table_state.select_column(Some(s.cursor_col));
        }
    }

    /// Cycle the projection mode of the current sheet (`m`): records ↔ key/value ↔
    /// scalars, whichever make sense for the anchored node.
    pub(crate) fn cycle_view_mode(&mut self) {
        let s = self.stack.active_mut();
        let Some(doc) = s.doc.as_mut() else {
            self.status_message = "Not a JSON/YAML/TOML sheet".to_string();
            return;
        };
        let modes = doc.available_modes();
        if modes.len() < 2 {
            self.status_message = "No other view for this node".to_string();
            return;
        }
        let cur = modes.iter().position(|m| *m == doc.view.mode).unwrap_or(0);
        let next = modes[(cur + 1) % modes.len()];
        match doc.set_mode(next) {
            Ok(df) => {
                s.dataframe = df;
                s.reset_view_state();
                self.status_message = format!("View: {}", mode_name(next));
            }
            Err(e) => self.status_message = e.to_string(),
        }
    }

    /// Expand the cursor column of containers into one column per child (`(`).
    pub(crate) fn expand_column(&mut self) {
        self.reshape_columns(true);
    }

    /// Fold the innermost expansion covering the cursor column back (`)`).
    pub(crate) fn contract_column(&mut self) {
        self.reshape_columns(false);
    }

    fn reshape_columns(&mut self, expand: bool) {
        let s = self.stack.active_mut();
        let col = s.cursor_col;
        let Some(doc) = s.doc.as_mut() else {
            self.status_message = "Not a JSON/YAML/TOML sheet".to_string();
            return;
        };
        let result = if expand {
            doc.expand_column(col)
        } else {
            doc.contract_column(col)
        };
        match result {
            Ok(df) => {
                let ncols = df.columns.len();
                s.dataframe = df;
                // The column is replaced in place, so keeping the cursor index lands on
                // the first child — which is what the user was looking at.
                s.cursor_col = col.min(ncols.saturating_sub(1));
                s.table_state.select_column(Some(s.cursor_col));
                // A sort on a column that may no longer exist cannot be kept.
                s.sort_col = None;
                s.left_col = s.left_col.min(s.cursor_col);
                self.status_message = format!("{} columns", ncols);
            }
            Err(e) => self.status_message = e.to_string(),
        }
    }
}

fn mode_name(m: crate::data::view::ViewMode) -> &'static str {
    use crate::data::view::ViewMode as V;
    match m {
        V::Records => "records",
        V::KeyValue => "key/value",
        V::Scalars => "scalars",
    }
}

/// Strip any breadcrumb trail already on a sheet title so diving twice does not produce
/// `file › a › file › a › b`.
fn root_name(title: &str) -> String {
    title.split(" › ").next().unwrap_or(title).to_string()
}
