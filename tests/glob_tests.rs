//! Patterns, directories and markdown pages — reading many files as one table.
//!
//! The three arrived together because they are one task: a directory of pages is only
//! reachable through a pattern, and a pattern is only worth having if what it matches
//! can be read.

use serde_json::{json, Value};
use std::path::PathBuf;
use tuitab::mcp::{handle_message, Server};

fn dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tmp")
        .join("glob-tests");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(rel: &str, text: &str) -> PathBuf {
    let path = dir().join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, text).unwrap();
    path
}

fn call(server: &mut Server, name: &str, arguments: Value) -> (bool, Value) {
    let response = handle_message(
        server,
        &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": {"name": name, "arguments": arguments}})
        .to_string(),
    )
    .expect("a request must get a response");
    let result = &response["result"];
    let text = result["content"][0]["text"].as_str().unwrap().to_string();
    let is_error = result["isError"].as_bool().unwrap_or(false);
    let body = serde_json::from_str(&text).unwrap_or(Value::String(text));
    (is_error, body)
}

fn rows(server: &mut Server, path: &str) -> (bool, Value) {
    call(
        server,
        "tuitab_query",
        json!({"source": {"path": path}, "ops": []}),
    )
}

#[test]
fn a_pattern_reads_every_file_it_matches() {
    let base = dir().join("csvs");
    let _ = std::fs::remove_dir_all(&base);
    write("csvs/one.csv", "a,b\n1,x\n2,y\n");
    write("csvs/two.csv", "a,b\n3,z\n");
    write("csvs/sub/three.csv", "a,b\n9,q\n");

    let mut server = Server::new();
    let (err, out) = rows(&mut server, &format!("{}/*.csv", base.display()));
    assert!(!err, "{}", out);
    assert_eq!(out["row_count"], json!(3), "{}", out);

    // `**` reaches the nested one, and the order is the sorted one, twice running.
    let deep = format!("{}/**/*.csv", base.display());
    let (err, out) = rows(&mut server, &deep);
    assert!(!err, "{}", out);
    assert_eq!(out["row_count"], json!(4));
    let (_, again) = rows(&mut server, &deep);
    assert_eq!(out["rows"], again["rows"], "a pattern is not a coin toss");

    // A relative pattern answers the same as the absolute one.
    let (err, relative) = rows(&mut server, "tmp/glob-tests/csvs/**/*.csv");
    assert!(!err, "{}", relative);
    assert_eq!(relative["rows"], out["rows"]);
}

#[test]
fn a_pattern_that_matches_nothing_says_so() {
    let mut server = Server::new();
    let (err, out) = rows(&mut server, "tmp/glob-tests/csvs/nothing-*.csv");
    assert!(err);
    // Not "No such file": the pattern is fine and nothing matched it, and the two call
    // for different next moves.
    assert!(
        out.as_str().unwrap().contains("glob matched no files"),
        "{}",
        out
    );
}

#[test]
fn a_file_with_other_columns_is_named_rather_than_stacked() {
    let base = dir().join("mismatch");
    let _ = std::fs::remove_dir_all(&base);
    write("mismatch/good.csv", "a,b\n1,x\n");
    write("mismatch/odd.csv", "c\n7\n");

    let mut server = Server::new();
    let (err, out) = rows(&mut server, &format!("{}/*.csv", base.display()));
    assert!(err, "{}", out);
    let message = out.as_str().unwrap();
    assert!(message.contains("odd.csv"), "{}", message);
    assert!(message.contains("different columns"), "{}", message);
}

#[test]
fn a_directory_is_a_list_of_its_files() {
    let base = dir().join("mixed");
    let _ = std::fs::remove_dir_all(&base);
    write("mixed/data.csv", "a\n1\n");
    write("mixed/cover.webp", "not an image, near enough");

    // It used to be handed to the CSV reader, which refused a directory holding two
    // extensions and advised a glob pattern the server then would not accept.
    let mut server = Server::new();
    let (err, out) = call(
        &mut server,
        "tuitab_inspect",
        json!({"source": {"path": base.to_string_lossy()}}),
    );
    assert!(!err, "{}", out);
    let names: Vec<&str> = out["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Name"), "{:?}", names);
    assert_eq!(out["row_count"], json!(2), "{}", out);
}

#[test]
fn a_page_is_a_row_of_its_frontmatter() {
    let base = dir().join("site");
    let _ = std::fs::remove_dir_all(&base);
    write(
        "site/bai-hao/index.md",
        "---\ntitle: Bai Hao\nprice: 12.5\n---\n\nBody one.\n\n---\n\nStill body.\n",
    );
    // A page with a field the first one lacks, and TOML frontmatter besides: a real
    // site has both, and a table of pages that refused them would be no use.
    write(
        "site/shou-mei/index.md",
        "+++\ntitle = \"Shou Mei\"\nprice = 9.0\ndraft = true\n+++\n\nBody two.\n",
    );

    let mut server = Server::new();
    let (err, out) = rows(&mut server, &format!("{}/*/index.md", base.display()));
    assert!(!err, "{}", out);
    assert_eq!(out["row_count"], json!(2), "{}", out);

    let names: Vec<&str> = out["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    // `file` first: with a pattern it is the only thing telling the rows apart.
    assert_eq!(names[0], "file");
    for wanted in ["title", "price", "draft", "body"] {
        assert!(names.contains(&wanted), "{:?} misses {}", names, wanted);
    }

    // The body keeps its own `---`, which is a rule and not the end of anything.
    let first = &out["rows"][0];
    assert!(
        first[names.iter().position(|n| *n == "body").unwrap()]
            .as_str()
            .unwrap()
            .contains("Still body"),
        "{}",
        first
    );
    // The field only the second page has is NULL on the first, not an error.
    assert!(first[names.iter().position(|n| *n == "draft").unwrap()].is_null());

    // And the point of the exercise: arithmetic over a directory of pages.
    let (err, total) = call(
        &mut server,
        "tuitab_query",
        json!({"source": {"path": format!("{}/*/index.md", base.display())},
               "ops": [{"aggregate": [{"col": "price", "fn": "sum"}]}]}),
    );
    assert!(!err, "{}", total);
    assert_eq!(total["rows"][0][0], json!(21.5));
}
