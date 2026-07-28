//! Not part of the normal suite: a measurement of where the in-memory document model
//! starts to hurt, so decisions about streaming rest on a number rather than a feeling.
//! Run with: cargo test --release --test doc_bench -- --ignored --nocapture

use std::path::Path;
use std::time::Instant;

#[test]
#[ignore = "benchmark; needs fixtures generated separately"]
fn measure_document_load() {
    let home = std::env::var("HOME").unwrap();
    for name in ["bench-50.json", "bench-200.json"] {
        let path = Path::new(&home).join(".tmp/tuitab-bench").join(name);
        if !path.exists() {
            println!("{}: missing, skipped", name);
            continue;
        }
        let size = std::fs::metadata(&path).unwrap().len() as f64 / 1024.0 / 1024.0;

        let t = Instant::now();
        let (df, doc) = tuitab::data::io::load_file_with_doc(&path, None).unwrap();
        let load = t.elapsed();

        let t = Instant::now();
        let text = doc
            .as_ref()
            .unwrap()
            .doc
            .read()
            .unwrap()
            .to_string_as(
                tuitab::data::doc::Format::Json,
                &tuitab::data::doc::SaveOpts::default(),
            )
            .unwrap();
        let save = t.elapsed();

        println!(
            "{:>16}  {:>6.1} MB  {:>7} rows  load {:>7.0?}  serialise {:>7.0?}  out {:.1} MB",
            name,
            size,
            df.visible_row_count(),
            load,
            save,
            text.len() as f64 / 1024.0 / 1024.0
        );
    }
}
