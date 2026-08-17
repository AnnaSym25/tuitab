use ratatui::widgets::ScrollbarState;

use crate::app::App;
use crate::data::typed_value::{parse_typed_value, TypedCell};
use crate::types::{Action, AppMode};
use crate::ui::text_input::TextInput;

impl App {
    pub(crate) fn handle_row_form_action(&mut self, action: Action) -> Option<Action> {
        // "Enter again to insert" may only be answered by that Enter.  Anything else —
        // a keystroke, a move, another action entirely — is the user doing something
        // other than agreeing, and the offer lapses.
        if !matches!(action, Action::ApplyRowForm) {
            self.row_form.confirm_empty = false;
        }
        match action {
            Action::OpenRowForm => {
                self.open_row_form();
                None
            }
            Action::RowFormInput(c) => {
                self.row_form_edit(|f| f.insert_char(c));
                None
            }
            Action::RowFormBackspace => {
                self.row_form_edit(TextInput::delete_backward);
                None
            }
            Action::RowFormForwardDelete => {
                self.row_form_edit(TextInput::delete_forward);
                None
            }
            // Moving the cursor changes nothing to check, so these skip revalidation.
            Action::RowFormCursorLeft => {
                self.row_form_focused(TextInput::move_cursor_left);
                None
            }
            Action::RowFormCursorRight => {
                self.row_form_focused(TextInput::move_cursor_right);
                None
            }
            Action::RowFormCursorStart => {
                self.row_form_focused(TextInput::move_cursor_start);
                None
            }
            Action::RowFormCursorEnd => {
                self.row_form_focused(TextInput::move_cursor_end);
                None
            }
            Action::RowFormFieldUp => {
                let n = self.row_form.fields.len();
                if n > 0 {
                    self.row_form.focus = (self.row_form.focus + n - 1) % n;
                }
                None
            }
            Action::RowFormFieldDown => {
                let n = self.row_form.fields.len();
                if n > 0 {
                    self.row_form.focus = (self.row_form.focus + 1) % n;
                }
                None
            }
            Action::ApplyRowForm => {
                self.apply_row_form();
                None
            }
            Action::CancelRowForm => {
                self.row_form.fields.clear();
                self.row_form.errors.clear();
                self.row_form.focus = 0;
                self.mode = AppMode::Normal;
                self.status_message.clear();
                None
            }
            other => Some(other),
        }
    }

    /// `O` — one empty field per column, nothing pre-filled.
    ///
    /// The guards are the same three `add_empty_row` applies, and they run here rather
    /// than at Enter: putting a form up on a sheet that cannot take a row and only then
    /// refusing it is worse than never opening it.
    fn open_row_form(&mut self) {
        if self.reject_on_doc_sheet("Adding a row") {
            return;
        }
        if self.stack.active().is_dir_sheet {
            self.status_message = "This is a directory listing".to_string();
            return;
        }
        let n = self.stack.active().dataframe.columns.len();
        if n == 0 {
            self.status_message = "Add a column first with 'zi'".to_string();
            return;
        }

        self.row_form.fields = vec![TextInput::new(); n];
        self.row_form.errors = vec![None; n];
        self.row_form.focus = 0;
        self.mode = AppMode::RowForm;
        self.row_form.confirm_empty = false;
        self.status_message =
            "New row: ↑↓/Tab field, ←→ cursor, Enter insert, Esc cancel".to_string();
    }

    /// Run `f` on the focused field without touching its validity.
    fn row_form_focused(&mut self, f: impl FnOnce(&mut TextInput)) {
        if let Some(field) = self.row_form.fields.get_mut(self.row_form.focus) {
            f(field);
        }
    }

    /// Check field `i` against its column.
    ///
    /// An empty field is a NULL, and on a database sheet so is the `\N` the cell
    /// editor uses — someone who learned the sentinel from `e` should not find it
    /// stored here as two characters.
    fn check_field(
        &self,
        i: usize,
        col_type: crate::types::ColumnType,
    ) -> Result<Option<TypedCell>, String> {
        let raw = self.row_form.fields[i].as_str();
        if self.stack.active().dataframe.db_rows.is_some()
            && raw.trim() == crate::data::dataframe::NULL_INPUT
        {
            return Ok(None);
        }
        parse_typed_value(raw, col_type)
    }

    /// Run `f` on the focused field, then recheck that one field — the point of the
    /// form is that the complaint arrives while the user is still on the field.
    fn row_form_edit(&mut self, f: impl FnOnce(&mut TextInput)) {
        self.row_form_focused(f);
        let i = self.row_form.focus;
        let Some(col_type) = self
            .stack
            .active()
            .dataframe
            .columns
            .get(i)
            .map(|c| c.col_type)
        else {
            return;
        };
        self.row_form.errors[i] = self.check_field(i, col_type).err();
    }

    /// Fields left blank.  `\N` is not one of them — that is a NULL the user typed.
    pub(crate) fn count_empty_fields(&self) -> usize {
        self.row_form
            .fields
            .iter()
            .filter(|f| f.as_str().trim().is_empty())
            .count()
    }

    /// Enter — check every field, then add the row at the end of the table.
    fn apply_row_form(&mut self) {
        let columns = &self.stack.active().dataframe.columns;
        if self.row_form.fields.len() != columns.len() {
            // The sheet changed under the form; there is nothing sensible to insert.
            self.mode = AppMode::Normal;
            self.status_message = "The table changed — press O again".to_string();
            return;
        }

        let col_types: Vec<_> = columns.iter().map(|c| c.col_type).collect();
        let mut values: Vec<Option<TypedCell>> = Vec::with_capacity(col_types.len());
        let mut first_bad: Option<usize> = None;
        let mut bad = 0usize;
        for (i, col_type) in col_types.iter().enumerate() {
            match self.check_field(i, *col_type) {
                Ok(v) => {
                    self.row_form.errors[i] = None;
                    values.push(v);
                }
                Err(e) => {
                    self.row_form.errors[i] = Some(e);
                    first_bad.get_or_insert(i);
                    bad += 1;
                    values.push(None);
                }
            }
        }

        if let Some(i) = first_bad {
            self.row_form.focus = i;
            self.status_message = if bad == 1 {
                format!("'{}' is not valid for this column", columns[i].name)
            } else {
                format!("{} fields need fixing", bad)
            };
            return;
        }

        // Every field is acceptable, but a blank one is a NULL rather than a value the
        // user chose.  Saying so once — and taking the next Enter as the answer — keeps
        // a half-filled row from going in on a keystroke meant to finish the field.
        let empty = self.count_empty_fields();
        if empty > 0 && !self.row_form.confirm_empty {
            self.row_form.confirm_empty = true;
            self.status_message = format!(
                "{} of {} fields empty — they go in as NULL.  Enter again to insert.",
                empty,
                col_types.len()
            );
            return;
        }
        self.row_form.confirm_empty = false;

        let s = self.stack.active_mut();
        s.push_undo();
        match s.dataframe.insert_rows_typed(&[values]) {
            Ok(()) => {
                let vis = s.dataframe.visible_row_count();
                s.scroll_state = ScrollbarState::new(vis.saturating_sub(1));
                s.table_state.select(Some(vis.saturating_sub(1)));
                self.row_form.fields.clear();
                self.row_form.errors.clear();
                self.row_form.focus = 0;
                self.mode = AppMode::Normal;
                self.status_message = "Added a row".to_string();
            }
            Err(e) => {
                // Same rollback as `add_empty_row`: put the frame back, then drop the
                // redo entry that restoring it just created.
                s.pop_undo();
                s.redo_stack.pop();
                self.status_message = format!("Could not add the row: {}", e);
            }
        }
    }
}
