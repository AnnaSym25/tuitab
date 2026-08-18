//! What `open_target` decides a path is — the one answer every caller that puts a file
//! on the sheet stack now shares, so the background loader cannot disagree with the
//! foreground one about whether something is a database (#43).

use std::path::{Path, PathBuf};
use tuitab::data::io::open_target;

fn dir() -> PathBuf {
    let d = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tmp")
        .join("open-target-tests");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn sqlite_at(name: &str) -> PathBuf {
    let path = dir().join(name);
    let _ = std::fs::remove_file(&path);
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch(
            "CREATE TABLE users (id INTEGER, name TEXT); INSERT INTO users VALUES (1,'ann');",
        )
        .unwrap();
    path
}

fn xlsx_at(name: &str, sheets: &[&str]) -> PathBuf {
    use rust_xlsxwriter::Workbook;
    let path = dir().join(name);
    let _ = std::fs::remove_file(&path);
    let mut wb = Workbook::new();
    for s in sheets {
        let ws = wb.add_worksheet().set_name(*s).unwrap();
        ws.write_string(0, 0, "col").unwrap();
        ws.write_number(1, 0, 1.0).unwrap();
    }
    wb.save(&path).unwrap();
    path
}

#[test]
fn a_database_opens_as_a_listing_that_knows_where_it_came_from() {
    let path = sqlite_at("plain.db");
    let opened = open_target(&path, None, None).unwrap();
    assert_eq!(opened.df.columns[0].name, "Table");
    assert_eq!(opened.sqlite_db_path.as_deref(), Some(path.as_path()));
    assert!(opened.duckdb_db_path.is_none() && opened.xlsx_db_path.is_none());
}

#[test]
fn a_workbook_of_several_sheets_opens_as_a_listing() {
    let path = xlsx_at("many.xlsx", &["alpha", "beta"]);
    let opened = open_target(&path, None, None).unwrap();
    assert_eq!(opened.df.columns[0].name, "Sheet");
    assert_eq!(opened.xlsx_db_path.as_deref(), Some(path.as_path()));
}

#[test]
fn a_workbook_of_one_sheet_opens_as_its_rows() {
    let path = xlsx_at("one.xlsx", &["only"]);
    let opened = open_target(&path, None, None).unwrap();
    assert_eq!(opened.df.columns[0].name, "col");
    assert!(
        opened.xlsx_db_path.is_none(),
        "a listing of length one stands between the user and their data"
    );
}

#[test]
fn an_explicit_type_wins_over_the_extension() {
    let path = dir().join("notes.db");
    let _ = std::fs::remove_file(&path);
    std::fs::write(&path, "a: 1\nb: two\n").unwrap();
    let opened = open_target(&path, None, Some(tuitab::data::doc::Format::Yaml)).unwrap();
    assert!(
        opened.doc.is_some(),
        "--type yaml must parse, not open a db"
    );
    assert!(opened.sqlite_db_path.is_none());
}

#[test]
fn an_explicit_type_beats_an_extension_that_would_otherwise_win() {
    // `.txt` has a loader of its own, so this is the case where the two really compete.
    let path = dir().join("records.txt");
    std::fs::write(
        &path,
        "{\"a\": 1, \"b\": \"two\"}\n{\"a\": 2, \"b\": \"three\"}\n",
    )
    .unwrap();

    let as_named = open_target(&path, None, None).unwrap();
    assert_ne!(
        as_named.df.columns.len(),
        2,
        "without --type this is plain text, one column"
    );

    let forced = open_target(&path, None, Some(tuitab::data::doc::Format::Jsonl)).unwrap();
    let cols: Vec<&str> = forced.df.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(cols, vec!["a", "b"]);
}

#[test]
fn a_plain_table_carries_no_container_path() {
    let path = Path::new("test_data/sample.csv");
    let opened = open_target(path, None, None).unwrap();
    assert!(
        opened.sqlite_db_path.is_none()
            && opened.duckdb_db_path.is_none()
            && opened.xlsx_db_path.is_none()
    );
}
