//! Running jq programs over a document.
//!
//! The result of a query is just another document, which is why this is a small module
//! rather than a subsystem: whatever comes back goes straight into
//! [`crate::data::io::doc_io::DocState::from_doc`] and becomes an ordinary sheet with
//! diving, editing and saving already working.

use crate::data::doc::Node;
use color_eyre::{eyre::eyre, Result};

/// Upper bound on collected outputs.  `range(1e9)` is a legal jq program.
const MAX_OUTPUTS: usize = 100_000;

/// Run `program` over `root` and return the result.
///
/// A single output is returned as itself; several become an array, which is what jq's
/// own `-s` would give and what a table wants anyway.
///
/// Values cross the jaq boundary as JSON text.  It costs a serialise and a parse on a
/// whole document, which is measurable on a 50 MB file — but it keeps this module
/// independent of jaq's internal value representation, and querying documents that size
/// is not the common case.
pub fn run_jq(root: &Node, program: &str) -> Result<Node> {
    use jaq_core::load::{Arena, File, Loader};
    use jaq_core::{data, unwrap_valr, Compiler, Ctx, Vars};
    use jaq_json::Val;

    let program = program.trim();
    if program.is_empty() {
        return Err(eyre!("empty query"));
    }

    let input_text = crate::data::doc::serialize(
        root,
        crate::data::doc::Format::Json,
        false,
        &crate::data::doc::SaveOpts {
            indent: false,
            sort_keys: false,
        },
    )?;
    let input = jaq_json::read::parse_single(input_text.as_bytes())
        .map_err(|e| eyre!("could not read the document as JSON: {e}"))?;

    let defs = jaq_core::defs()
        .chain(jaq_std::defs())
        .chain(jaq_json::defs());
    let funs = jaq_core::funs()
        .chain(jaq_std::funs())
        .chain(jaq_json::funs());

    let loader = Loader::new(defs);
    let arena = Arena::default();
    let modules = loader
        .load(&arena, File { code: program, path: () })
        .map_err(|_| eyre!("could not parse `{program}`"))?;
    let filter = Compiler::default()
        .with_funs(funs)
        .compile(modules)
        .map_err(|_| eyre!("could not compile `{program}`"))?;

    let ctx = Ctx::<data::JustLut<Val>>::new(&filter.lut, Vars::new([]));
    let mut outputs = Vec::new();
    for result in filter.id.run((ctx, input)).map(unwrap_valr) {
        let val = result.map_err(|e| eyre!("{e}"))?;
        outputs.push(parse_val(&val)?);
        if outputs.len() >= MAX_OUTPUTS {
            break;
        }
    }

    match outputs.len() {
        // No output is a legitimate jq result — `select` that matches nothing is the
        // usual way to ask "are there any?" — so it opens an empty sheet rather than
        // erroring.  Only a program that cannot run is a failure.
        0 => Ok(Node::Arr(Vec::new())),
        1 => Ok(outputs.pop().expect("length checked")),
        _ => Ok(Node::Arr(outputs)),
    }
}

fn parse_val(val: &jaq_json::Val) -> Result<Node> {
    let doc = crate::data::doc::Doc::from_str(&val.to_string(), crate::data::doc::Format::Json)?;
    Ok(doc.root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::doc::{Doc, Format, Seg};

    fn root(src: &str) -> Node {
        Doc::from_str(src, Format::Json).unwrap().root
    }

    #[test]
    fn a_query_selecting_records_comes_back_as_an_array() {
        let r = root(r#"[{"n":1,"ok":true},{"n":2,"ok":false},{"n":3,"ok":true}]"#);
        let out = run_jq(&r, ".[] | select(.ok)").unwrap();
        let Node::Arr(items) = &out else {
            panic!("expected an array, got {:?}", out)
        };
        assert_eq!(items.len(), 2);
        assert_eq!(
            out.get(&[Seg::Idx(1), Seg::Key("n".into())]),
            Some(&Node::Int(3))
        );
    }

    #[test]
    fn a_single_output_is_returned_as_itself() {
        let r = root(r#"{"a":{"b":[1,2]}}"#);
        assert_eq!(run_jq(&r, ".a.b").unwrap(), root("[1,2]"));
        assert_eq!(run_jq(&r, ".a.b | length").unwrap(), Node::Int(2));
    }

    #[test]
    fn queries_can_reshape_into_something_tabular() {
        let r = root(r#"{"users":{"alice":{"age":30},"bob":{"age":40}}}"#);
        let out = run_jq(&r, ".users | to_entries | map({name: .key, age: .value.age})").unwrap();
        assert_eq!(
            out.get(&[Seg::Idx(0), Seg::Key("name".into())]),
            Some(&Node::Str("alice".into()))
        );
        assert_eq!(
            out.get(&[Seg::Idx(1), Seg::Key("age".into())]),
            Some(&Node::Int(40))
        );
    }

    #[test]
    fn a_bad_program_is_reported_rather_than_panicking() {
        let r = root("[1,2]");
        assert!(run_jq(&r, ".[ | broken").is_err());
        assert!(run_jq(&r, "").is_err());
    }

    #[test]
    fn a_program_matching_nothing_yields_an_empty_result_not_an_error() {
        let r = root("[1,2]");
        assert_eq!(run_jq(&r, ".[] | select(. > 99)").unwrap(), Node::Arr(vec![]));
    }

    #[test]
    fn a_runtime_error_is_reported_rather_than_panicking() {
        let r = root(r#"{"a":1}"#);
        assert!(run_jq(&r, ".a | .[0]").is_err(), "indexing a number");
    }
}
