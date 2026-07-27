use crate::app::App;
use crate::types::{Action, AppMode};

impl App {
    pub(crate) fn handle_io_action(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::SaveFile => {
                self.save.error = None;
                let default_path = self
                    .stack
                    .active()
                    .source_path
                    .as_deref()
                    .and_then(|p| {
                        std::env::current_dir()
                            .ok()
                            .and_then(|cwd| {
                                p.strip_prefix(&cwd)
                                    .ok()
                                    .map(|r| r.to_string_lossy().into_owned())
                            })
                            .or_else(|| Some(p.to_string_lossy().into_owned()))
                    })
                    .unwrap_or_else(|| self.stack.active().title.clone());

                self.save.autocomplete_candidates.clear();
                self.save.autocomplete_prefix.clear();
                self.save.autocomplete_idx = 0;
                self.save.input = crate::ui::text_input::TextInput::with_value(default_path);
                self.mode = AppMode::Saving;
                None
            }
            Action::SavingInput(c) => {
                self.save.input.insert_char(c);
                None
            }
            Action::SavingBackspace => {
                self.save.input.delete_backward();
                None
            }
            Action::SavingForwardDelete => {
                self.save.input.delete_forward();
                None
            }
            Action::SavingCursorLeft => {
                self.save.input.move_cursor_left();
                None
            }
            Action::SavingCursorRight => {
                self.save.input.move_cursor_right();
                None
            }
            Action::SavingCursorStart => {
                self.save.input.move_cursor_start();
                None
            }
            Action::SavingCursorEnd => {
                self.save.input.move_cursor_end();
                None
            }
            Action::ApplySave => {
                // A path parked by the shape popup means the question has been answered
                // already; taking it here is what stops the two from bouncing forever.
                let answered = self.save.pending_path.take();
                let path = answered
                    .clone()
                    .unwrap_or_else(|| crate::app::expand_tilde(self.save.input.as_str()));
                // A table with no document behind it can become several different
                // documents, and only the user knows which one they meant — ask, once,
                // and remember the answer for the rest of the session.
                if answered.is_none() && self.needs_shape_for(&path) {
                    self.save.pending_path = Some(path);
                    self.save.shapes = crate::data::io::doc_io::Shape::options(
                        self.stack.active().dataframe.col_count(),
                    );
                    // Restore the cursor onto the remembered shape, not its old position.
                    self.save.shape_index = self
                        .save
                        .shapes
                        .iter()
                        .position(|s| *s == self.save.shape)
                        .unwrap_or(0);
                    self.mode = AppMode::SaveShapeSelect;
                    return None;
                }
                let shape = self.chosen_shape();
                let sheet = self.stack.active();
                // A doc-backed sheet is written by re-serialising its tree, so structure
                // survives and changing the extension converts between formats.
                let result = crate::data::io::save_file_as(
                    &sheet.dataframe,
                    sheet.doc.as_ref(),
                    &path,
                    shape,
                    &sheet.title,
                );
                match result {
                    Ok(_) => {
                        self.mode = AppMode::Normal;
                        self.status_message =
                            format!("Saved successfully to: {}", self.save.input.as_str());
                        self.save.error = None;
                    }
                    Err(e) => {
                        self.save.error = Some(format!("Error: {}", e));
                    }
                }
                None
            }
            Action::CancelSave => {
                self.mode = AppMode::Normal;
                self.save.error = None;
                None
            }
            Action::ChoiceUp => {
                self.save.shape_index = self.save.shape_index.saturating_sub(1);
                None
            }
            Action::ChoiceDown => {
                let len = if self.mode == AppMode::OpenAsSelect {
                    crate::app::OPEN_AS_FORMATS.len()
                } else {
                    self.save.shapes.len()
                };
                self.save.shape_index = (self.save.shape_index + 1).min(len.saturating_sub(1));
                None
            }
            Action::ApplySaveShape => {
                if let Some(s) = self.save.shapes.get(self.save.shape_index) {
                    self.save.shape = *s;
                }
                // Returning Some() here would only pass the action further down the
                // handler chain, not run it — the save has to be re-entered directly.
                self.mode = AppMode::Saving;
                self.handle_io_action(Action::ApplySave)
            }
            Action::CancelSaveShape => {
                self.save.pending_path = None;
                self.mode = AppMode::Saving;
                None
            }
            Action::OpenAs => {
                if self.stack.active().is_dir_sheet {
                    self.save.shape_index = 0;
                    self.save.shapes.clear();
                    self.mode = AppMode::OpenAsSelect;
                } else {
                    self.mode = AppMode::Normal;
                    self.status_message =
                        "Open as… works on a directory listing".to_string();
                }
                None
            }
            Action::ApplyOpenAs => {
                let fmt = crate::app::OPEN_AS_FORMATS[self.save.shape_index];
                self.mode = AppMode::Normal;
                self.open_directory_row_as(Some(fmt));
                None
            }
            Action::CancelOpenAs => {
                self.mode = AppMode::Normal;
                None
            }
            Action::SavingAutocomplete => {
                self.saving_autocomplete();
                None
            }
            other => Some(other),
        }
    }

    /// True when saving to `path` has to ask for a shape first: the target is one of
    /// the document formats and this sheet has no document to re-serialise.
    fn needs_shape_for(&self, path: &std::path::Path) -> bool {
        if self.stack.active().doc.is_some() {
            return false;
        }
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(crate::data::doc::Format::from_ext)
            .is_some()
    }

    fn chosen_shape(&self) -> crate::data::io::doc_io::Shape {
        self.save.shape
    }

    pub(super) fn saving_autocomplete(&mut self) {
        let input = self.save.input.as_str().to_owned();

        let path = std::path::Path::new(&input);
        let (dir, prefix) = if input.ends_with('/') {
            (path, "")
        } else {
            let dir = path.parent().unwrap_or(std::path::Path::new("."));
            let prefix = path
                .file_name()
                .map(|f| f.to_str().unwrap_or(""))
                .unwrap_or("");
            (dir, prefix)
        };

        let dir_str = if dir == std::path::Path::new("") {
            std::path::Path::new(".")
        } else {
            dir
        };
        let expanded_dir = crate::app::expand_tilde(dir_str.to_str().unwrap_or("."));

        let full_prefix = input.trim_end_matches(prefix).to_string();
        if self.save.autocomplete_prefix != full_prefix
            || self.save.autocomplete_candidates.is_empty()
        {
            self.save.autocomplete_prefix = full_prefix.clone();
            self.save.autocomplete_idx = 0;

            let mut candidates: Vec<String> = std::fs::read_dir(&expanded_dir)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    if is_dir {
                        format!("{}/", name)
                    } else {
                        name
                    }
                })
                .filter(|name| name.starts_with(prefix))
                .collect();

            candidates.sort();
            self.save.autocomplete_candidates = candidates;
        }

        if self.save.autocomplete_candidates.is_empty() {
            return;
        }

        let common = crate::app::longest_common_prefix(&self.save.autocomplete_candidates);
        let current_suffix = self
            .save
            .input
            .as_str()
            .strip_prefix(&self.save.autocomplete_prefix)
            .unwrap_or("");

        if common.len() > current_suffix.len() {
            let new_value = format!("{}{}", self.save.autocomplete_prefix, common);
            self.save.input = crate::ui::text_input::TextInput::with_value(new_value);
        } else {
            self.save.autocomplete_idx =
                (self.save.autocomplete_idx + 1) % self.save.autocomplete_candidates.len();
            let completion = &self.save.autocomplete_candidates[self.save.autocomplete_idx];
            let new_value = format!("{}{}", self.save.autocomplete_prefix, completion);
            self.save.input = crate::ui::text_input::TextInput::with_value(new_value);
        }
    }
}
