//! End-to-end checks for the JSON / JSONL / YAML / TOML pipeline: open a real file,
//! project it, edit through the tree, save it back, and convert between formats.

use std::path::{Path, PathBuf};
use tuitab::data::doc::{Format, Seg};
use tuitab::data::io::{doc_io::Shape, load_file_as, load_file_with_doc, save_file_as};
use tuitab::data::view::ViewMode;

fn fixture(name: &str) -> PathBuf {
    Path::new("test_data").join(name)
}

fn out(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("tuitab-doc-tests");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn toml_opens_as_key_value_rows_with_typed_nodes() {
    let (df, doc) = load_file_with_doc(&fixture("app.toml"), None).unwrap();
    let doc = doc.expect("toml must carry a document tree");
    assert_eq!(doc.view.mode, ViewMode::KeyValue);

    let names: Vec<&str> = df.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["key", "value", "type"]);
    assert_eq!(df.visible_row_count(), 5, "5 top-level keys");

    // the datetime is a real node type, not a string, and the nested table renders
    // compactly instead of being lost
    let types: Vec<String> = (0..5).map(|r| df.get_physical(r, 2)).collect();
    assert!(types.contains(&"datetime".to_string()), "{:?}", types);
    assert!(types.contains(&"dict".to_string()), "{:?}", types);
    assert!(types.contains(&"list".to_string()), "{:?}", types);
}

#[test]
fn editing_a_toml_value_and_saving_preserves_structure_and_datetime() {
    let (mut df, doc) = load_file_with_doc(&fixture("app.toml"), None).unwrap();
    let mut doc = doc.unwrap();

    // row 1 is `port`; write through the tree, then patch the table like the app does
    let row = (0..df.visible_row_count())
        .find(|r| df.get_physical(*r, 0) == "port")
        .unwrap();
    let shown = doc.set_cell(row, 1, "9090").unwrap();
    df.set_cell(row, 1, shown).unwrap();

    let path = out("app-edited.toml");
    save_file_as(&df, Some(&doc), &path, Shape::Records, "app").unwrap();
    let text = std::fs::read_to_string(&path).unwrap();

    assert!(text.contains("port = 9090"), "{}", text);
    assert!(
        text.contains("1979-05-27T07:32:00Z"),
        "datetime must not degrade to a quoted string: {}",
        text
    );
    assert!(text.contains("[db]"), "nested table survives: {}", text);
    assert!(
        text.contains("[[servers]]"),
        "array of tables survives: {}",
        text
    );
}

#[test]
fn toml_converts_to_yaml_and_json_by_changing_the_extension() {
    let (df, doc) = load_file_with_doc(&fixture("app.toml"), None).unwrap();
    let doc = doc.unwrap();

    let yaml_path = out("app.yaml");
    save_file_as(&df, Some(&doc), &yaml_path, Shape::Records, "app").unwrap();
    let yaml = std::fs::read_to_string(&yaml_path).unwrap();
    assert!(yaml.contains("host: localhost"), "{}", yaml);

    let json_path = out("app.json");
    save_file_as(&df, Some(&doc), &json_path, Shape::Records, "app").unwrap();
    let json = std::fs::read_to_string(&json_path).unwrap();
    assert!(json.contains("\"pool\": 10"), "{}", json);

    // and the converted file reopens as the same structure
    let (_, reopened) = load_file_with_doc(&yaml_path, None).unwrap();
    let reopened = reopened.unwrap();
    let host = reopened
        .doc
        .read()
        .unwrap()
        .root
        .get(&[Seg::Key("db".into()), Seg::Key("host".into())])
        .cloned();
    assert_eq!(host, Some(tuitab::data::doc::Node::Str("localhost".into())));
}

#[test]
fn multi_document_yaml_becomes_rows_and_stays_multi_document() {
    let (df, doc) = load_file_with_doc(&fixture("k8s.yaml"), None).unwrap();
    let doc = doc.unwrap();
    assert_eq!(doc.view.mode, ViewMode::Records);
    assert_eq!(df.visible_row_count(), 2, "one row per document");

    let path = out("k8s-out.yaml");
    save_file_as(&df, Some(&doc), &path, Shape::Records, "k8s").unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        text.matches("---").count(),
        2,
        "document separators must survive: {}",
        text
    );
}

#[test]
fn jsonl_round_trips_and_can_be_saved_as_json() {
    let (df, doc) = load_file_with_doc(&fixture("rows.jsonl"), None).unwrap();
    let doc = doc.unwrap();
    assert_eq!(df.visible_row_count(), 2);
    assert_eq!(df.col_count(), 2);

    let path = out("rows.json");
    save_file_as(&df, Some(&doc), &path, Shape::Records, "rows").unwrap();
    let json = std::fs::read_to_string(&path).unwrap();
    assert!(json.trim_start().starts_with('['), "{}", json);
}

#[test]
fn nested_json_dives_into_subtrees_and_edits_are_visible_from_the_root() {
    let (_df, doc) = load_file_with_doc(&fixture("nested.json"), None).unwrap();
    let root = doc.unwrap();

    // dive into row 0's `tags` list
    let (tags_df, mut tags) = root
        .dive(vec![Seg::Idx(0), Seg::Key("tags".into())])
        .unwrap();
    assert_eq!(tags.view.mode, ViewMode::Scalars);
    assert_eq!(tags_df.visible_row_count(), 2);
    tags.set_cell(0, 0, "z").unwrap();

    // the parent sees it, because both sheets share one tree
    let value = root
        .doc
        .read()
        .unwrap()
        .root
        .get(&[Seg::Idx(0), Seg::Key("tags".into()), Seg::Idx(0)])
        .cloned();
    assert_eq!(value, Some(tuitab::data::doc::Node::Str("z".into())));
}

#[test]
fn a_plain_csv_saves_as_json_records() {
    let (df, doc) = load_file_with_doc(&fixture("prices.csv"), None).unwrap();
    assert!(doc.is_none(), "csv has no document tree");

    let path = out("prices.json");
    save_file_as(&df, None, &path, Shape::Records, "prices").unwrap();
    let json = std::fs::read_to_string(&path).unwrap();
    assert!(json.trim_start().starts_with('['), "{}", json);

    // and as TOML it becomes an array of tables named after the sheet
    let toml_path = out("prices.toml");
    save_file_as(&df, None, &toml_path, Shape::Records, "prices").unwrap();
    let toml = std::fs::read_to_string(&toml_path).unwrap();
    assert!(toml.contains("[[prices]]"), "{}", toml);
}

#[test]
fn forcing_a_format_overrides_the_extension() {
    // prices.csv opened as CSV has real columns; the same path forced to YAML must fail
    // rather than silently producing nonsense
    let forced = load_file_as(&fixture("prices.csv"), None, Some(Format::Toml));
    assert!(forced.is_err(), "csv is not valid toml");

    let (_, doc) = load_file_as(&fixture("app.toml"), None, Some(Format::Toml)).unwrap();
    assert!(doc.is_some());
}

/// The whole path, headless: open a real TOML file, render the actual UI, and check the
/// key/value rows land on screen.  Catches wiring breaks that unit tests on the data
/// layer would miss.
#[test]
fn a_toml_file_renders_as_key_value_rows_in_the_real_ui() {
    use ratatui::{backend::TestBackend, Terminal};

    let mut app = tuitab::app::App::new(&fixture("app.toml"), None).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(120, 24)).unwrap();
    terminal
        .draw(|f| tuitab::ui::render(f, &mut app))
        .unwrap();

    let screen: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect();

    for expected in ["key", "value", "type", "port", "8080", "datetime", "{2} host=localhost"] {
        assert!(
            screen.contains(expected),
            "expected `{}` on screen:\n{}",
            expected,
            screen
        );
    }
}

/// Drive the app the way a user does: move to a nested key, press Enter to dive, press
/// `m` to change the projection, and pop back out.
#[test]
fn enter_dives_into_a_nested_node_and_esc_pops_back() {
    use tuitab::types::Action;

    let mut app = tuitab::app::App::new(&fixture("app.toml"), None).unwrap();
    let root_title = app.stack.active().title.clone();

    // walk down to the `servers` row
    while app.stack.active().dataframe.get_physical(
        app.stack.active().table_state.selected().unwrap_or(0),
        0,
    ) != "servers"
    {
        app.handle_action(Action::MoveDown);
    }

    app.handle_action(Action::OpenRow);
    assert_eq!(
        app.stack.active().title,
        "app.toml › servers",
        "breadcrumbs name the anchored node"
    );
    assert_eq!(app.stack.active().dataframe.visible_row_count(), 2);
    let cols: Vec<String> = app
        .stack
        .active()
        .dataframe
        .columns
        .iter()
        .map(|c| c.name.clone())
        .collect();
    assert_eq!(cols, vec!["host", "weight"], "array of tables → records");

    app.handle_action(Action::CycleViewMode);
    assert_ne!(
        app.stack.active().dataframe.columns[0].name,
        "host",
        "the projection actually changed"
    );

    app.handle_action(Action::PopSheet);
    assert_eq!(app.stack.active().title, root_title);
}

/// Regression: opening a structured file *through the directory listing* must keep its
/// document tree.  Without it, `Ctrl+S` over the same name rewrites a TOML config as a
/// flat array of tables.
#[test]
fn drilling_in_from_a_directory_keeps_the_document() {
    use tuitab::types::Action;

    let mut app = tuitab::app::App::new(Path::new("test_data"), None).unwrap();
    assert!(app.stack.active().is_dir_sheet);

    // find the app.toml row and open it
    let df = &app.stack.active().dataframe;
    let row = (0..df.visible_row_count())
        .find(|r| df.get_physical(*r, 0).contains("app.toml"))
        .expect("app.toml must be listed");
    app.stack.active_mut().table_state.select(Some(row));
    app.handle_action(Action::OpenRow);

    let sheet = app.stack.active();
    assert!(
        sheet.doc.is_some(),
        "a TOML opened from a directory must carry its tree"
    );

    let path = out("drill-save.toml");
    save_file_as(
        &sheet.dataframe,
        sheet.doc.as_ref(),
        &path,
        Shape::Records,
        &sheet.title,
    )
    .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("[db]"), "structure must survive: {}", text);
    assert!(text.contains("[[servers]]"), "{}", text);
    assert!(
        !text.contains("[[test_data"),
        "must not be rewritten as a flat table: {}",
        text
    );
}

/// Regression: an edit made inside a dive is visible on the parent sheet after `Esc`,
/// not just in the saved file.
#[test]
fn popping_back_from_a_dive_shows_the_edit() {
    use tuitab::types::Action;

    let mut app = tuitab::app::App::new(&fixture("nested.json"), None).unwrap();
    // row 0, column `meta` holds a container rendered as `{1} ok=true`
    let meta_col = app
        .stack
        .active()
        .dataframe
        .columns
        .iter()
        .position(|c| c.name == "meta")
        .unwrap();
    app.stack.active_mut().cursor_col = meta_col;
    assert_eq!(
        app.stack.active().dataframe.get_physical(0, meta_col),
        "{1} ok=true"
    );

    app.handle_action(Action::OpenCell);
    assert!(app.stack.can_pop(), "dived into the meta object");
    // key/value view: edit the `ok` value
    {
        let s = app.stack.active_mut();
        s.edit_row = 0;
        s.edit_col = 1;
        s.edit_input = tuitab::ui::text_input::TextInput::with_value("false".into());
    }
    app.handle_action(Action::ApplyEdit);
    app.handle_action(Action::PopSheet);

    assert_eq!(
        app.stack.active().dataframe.get_physical(0, meta_col),
        "{1} ok=false",
        "the parent must re-render the edited subtree"
    );
}

/// Regression: a records array whose objects have a key literally named `value`, mixed
/// with bare scalar rows, must not confuse that key with the synthetic bare-value column.
#[test]
fn a_key_named_value_is_not_mistaken_for_the_bare_column() {
    use tuitab::data::doc::{Doc, Node};
    use tuitab::data::io::doc_io::DocState;

    let doc = Doc::from_str(r#"[{"value":"real"}, 7]"#, Format::Json).unwrap();
    let (df, mut state) = DocState::from_doc(doc).unwrap();
    let names: Vec<&str> = df.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["value_1", "value"], "the synthetic column is renamed");

    // editing the real `value` key must change only that key, not the whole row
    state.set_cell(0, 1, "changed").unwrap();
    let root = state.doc.read().unwrap().root.clone();
    assert_eq!(
        root.get(&[Seg::Idx(0)]).map(|n| n.type_name()),
        Some("dict"),
        "row 0 must still be an object"
    );
    assert_eq!(
        root.get(&[Seg::Idx(0), Seg::Key("value".into())]),
        Some(&Node::Str("changed".into()))
    );
}
