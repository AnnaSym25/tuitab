//! Markdown with frontmatter, read as one row.
//!
//! A page in a static site is a record whose fields live in the frontmatter and whose
//! body is one more field.  Read that way, a directory of them is a table — and with a
//! pattern, `content/**/index.md` is the whole site in one call, which is what makes
//! reconciling a site against a database possible without an export script in between.
//!
//! The frontmatter is parsed by tuitab's own document reader, so YAML (`---`) and TOML
//! (`+++`) behave exactly as they do in a `.yaml` or `.toml` file, nesting included.

use crate::data::dataframe::DataFrame;
use crate::data::doc::{Doc, Format, SaveOpts};
use color_eyre::{eyre::eyre, Result};
use std::path::Path;

/// Where the frontmatter ends and the page begins.
///
/// A fence is the first line of the file and its exact repeat later; anything before
/// the first line is not frontmatter, and a file without one is all body.
fn split(text: &str) -> (Option<(Format, &str)>, &str) {
    for (fence, format) in [("---", Format::Yaml), ("+++", Format::Toml)] {
        let Some(rest) = text.strip_prefix(fence) else {
            continue;
        };
        let Some(rest) = rest.strip_prefix('\n') else {
            continue;
        };
        let closing = format!("\n{}", fence);
        if let Some(end) = rest.find(&closing) {
            let after = &rest[end + closing.len()..];
            let body = after.strip_prefix('\n').unwrap_or(after);
            return (Some((format, &rest[..end])), body);
        }
    }
    (None, text)
}

pub(super) fn load_markdown(path: &Path) -> Result<DataFrame> {
    let text = std::fs::read_to_string(path)?;
    let (front, body) = split(&text);

    let mut row = serde_json::Map::new();
    // First, and named after the file it came from: with a pattern this is the only
    // thing telling one page's row from another's.
    row.insert(
        "file".to_string(),
        serde_json::Value::String(path.display().to_string()),
    );
    if let Some((format, source)) = front {
        let parsed = Doc::from_str(source, format)
            .and_then(|doc| doc.to_string_as(Format::Json, &SaveOpts::default()))
            .map_err(|e| eyre!("frontmatter: {}", e))?;
        if let Ok(serde_json::Value::Object(fields)) = serde_json::from_str(&parsed) {
            for (k, v) in fields {
                row.insert(k, v);
            }
        }
    }
    row.insert(
        "body".to_string(),
        serde_json::Value::String(body.trim().to_string()),
    );

    // Through the document reader as a one-row list of records, so nested frontmatter
    // flattens the way the same structure would in a .json file.
    let as_records = serde_json::Value::Array(vec![serde_json::Value::Object(row)]);
    let doc = Doc::from_str(&as_records.to_string(), Format::Json)?;
    let (df, _) = super::doc_io::DocState::from_doc(doc)?;
    Ok(df)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fence_is_the_first_line_and_its_repeat() {
        let (front, body) = split("---\ntitle: Tea\n---\nBody here\n");
        assert!(matches!(front, Some((Format::Yaml, "title: Tea"))));
        assert_eq!(body, "Body here\n");

        let (front, body) = split("+++\ntitle = \"Tea\"\n+++\nBody\n");
        assert!(matches!(front, Some((Format::Toml, "title = \"Tea\""))));
        assert_eq!(body, "Body\n");

        // No fence at all, and a fence that never closes: all body either way.
        assert!(split("# Just a heading\n").0.is_none());
        assert!(split("---\ntitle: Tea\nnever closed\n").0.is_none());

        // A rule inside the body is not a closing fence for a file with no frontmatter.
        let (front, body) = split("Text\n\n---\n\nMore\n");
        assert!(front.is_none());
        assert_eq!(body, "Text\n\n---\n\nMore\n");
    }
}
