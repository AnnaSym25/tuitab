use crate::data::dataframe::DataFrame;
use crate::data::io::{doc_io::DocState, load_file_with_doc};
use color_eyre::Result;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

/// Event sent from the background loading thread to the main thread.
pub enum LoadEvent {
    /// The loaded table, plus the document tree when the file was a structured format.
    /// The tree has to travel with it: a sheet without one silently loses nesting the
    /// moment it is saved.
    Complete(Result<(DataFrame, Option<DocState>)>),
}

/// Spawn a background thread to load a file.
/// Returns a `Receiver` that delivers a `LoadEvent::Complete` when done.
pub fn load_in_background(path: PathBuf, delimiter: Option<u8>) -> mpsc::Receiver<LoadEvent> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = load_file_with_doc(&path, delimiter);
        let _ = tx.send(LoadEvent::Complete(result));
    });

    rx
}
