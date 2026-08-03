//! The pipeline of operations a model sends to `tuitab_query`.
//!
//! Each operation maps onto a function that already exists in [`crate::data`],
//! so the arithmetic here is tuitab's, not a second implementation of it.  The
//! only genuinely new compute is `filter`.
//!
//! # How operations compose
//!
//! A [`DataFrame`] keeps `row_order`, a display-order permutation over the
//! physical rows, and every reshaping function in `crate::data` reads its input
//! through [`DataFrame::get_visible_df`].  That gives the composition rule:
//!
//! * `filter`, `limit` and `sort` rewrite `row_order` and move no data;
//! * `select` and `compute` change columns and leave `row_order` alone;
//! * `group_by`, `frequency`, `pivot` and `join` read through `get_visible_df`
//!   and return a fresh frame whose `row_order` is the identity.
//!
//! So `filter` then `group_by` aggregates only the surviving rows, and `filter`
//! then `sort` sorts only them — without any operation having to materialise a
//! copy just to hand it to the next one.

use crate::data::aggregator::AggregatorKind;
use crate::data::column::ColumnMeta;
use crate::data::dataframe::DataFrame;
use crate::data::expression::Expr as TuiExpr;
use crate::data::join::{join_dataframes, JoinType};
use crate::types::ColumnType;
use polars::prelude::*;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;

// ── Operation model ─────────────────────────────────────────────────────────

pub enum Op {
    Filter(Vec<Predicate>),
    Select(Vec<String>),
    Sort {
        col: String,
        desc: bool,
    },
    Compute {
        name: String,
        expr: String,
    },
    GroupBy {
        by: Vec<String>,
        agg: Vec<AggSpec>,
    },
    Frequency {
        by: Vec<String>,
        agg: Vec<AggSpec>,
    },
    Pivot {
        index: Vec<String>,
        on: String,
        formula: String,
    },
    Join(JoinSpec),
    Limit(usize),
}

pub struct AggSpec {
    pub col: String,
    pub kind: AggregatorKind,
}

pub struct JoinSpec {
    pub source: super::source::Source,
    pub left_on: Vec<String>,
    pub right_on: Vec<String>,
    pub how: JoinType,
}

pub struct Predicate {
    pub col: String,
    pub op: PredOp,
    pub value: Value,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PredOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    In,
    NotIn,
    Contains,
    Between,
    IsEmpty,
    NotEmpty,
}

impl PredOp {
    fn parse(s: &str) -> Result<Self, String> {
        Ok(match s {
            "eq" => Self::Eq,
            "ne" => Self::Ne,
            "gt" => Self::Gt,
            "ge" => Self::Ge,
            "lt" => Self::Lt,
            "le" => Self::Le,
            "in" => Self::In,
            "not_in" => Self::NotIn,
            "contains" => Self::Contains,
            "between" => Self::Between,
            "is_empty" => Self::IsEmpty,
            "not_empty" => Self::NotEmpty,
            other => {
                return Err(format!(
                    "Unknown filter operator '{}'. Available: eq, ne, gt, ge, lt, le, \
                     in, not_in, contains, between, is_empty, not_empty",
                    other
                ))
            }
        })
    }
}

// ── Parsing ─────────────────────────────────────────────────────────────────

/// Parse the `ops` array.  Each entry is a single-key object naming the
/// operation, e.g. `{"sort": {"col": "amount", "desc": true}}`.
pub fn parse_ops(value: &Value) -> Result<Vec<Op>, String> {
    let array = value
        .as_array()
        .ok_or_else(|| "'ops' must be an array of operations".to_string())?;

    array
        .iter()
        .enumerate()
        .map(|(i, v)| parse_op(v).map_err(|e| format!("ops[{}]: {}", i, e)))
        .collect()
}

fn parse_op(value: &Value) -> Result<Op, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "an operation must be an object like {\"limit\": 10}".to_string())?;

    if obj.len() != 1 {
        return Err(format!(
            "an operation must have exactly one key naming it, found {}",
            obj.len()
        ));
    }

    let (name, body) = obj.iter().next().expect("checked len == 1");

    match name.as_str() {
        "filter" => Ok(Op::Filter(parse_predicates(body)?)),
        "select" => Ok(Op::Select(string_array(body, "select")?)),
        "sort" => Ok(Op::Sort {
            col: required_str(body, "col")?,
            desc: body.get("desc").and_then(Value::as_bool).unwrap_or(false),
        }),
        "compute" => Ok(Op::Compute {
            name: required_str(body, "name")?,
            expr: required_str(body, "expr")?,
        }),
        "group_by" => Ok(Op::GroupBy {
            by: string_array(body.get("by").unwrap_or(&Value::Null), "by")?,
            agg: parse_aggs(body.get("agg"))?,
        }),
        "frequency" => Ok(Op::Frequency {
            by: string_array(body.get("by").unwrap_or(&Value::Null), "by")?,
            agg: parse_aggs(body.get("agg"))?,
        }),
        "pivot" => Ok(Op::Pivot {
            index: string_array(body.get("index").unwrap_or(&Value::Null), "index")?,
            on: required_str(body, "on")?,
            formula: required_str(body, "formula")?,
        }),
        "join" => Ok(Op::Join(parse_join(body)?)),
        "limit" => body
            .as_u64()
            .or_else(|| body.get("n").and_then(Value::as_u64))
            .map(|n| Op::Limit(n as usize))
            .ok_or_else(|| "'limit' takes a number".to_string()),
        other => Err(format!(
            "Unknown operation '{}'. Available: filter, select, sort, compute, \
             group_by, frequency, pivot, join, limit",
            other
        )),
    }
}

fn required_str(body: &Value, key: &str) -> Result<String, String> {
    body.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("missing required string field '{}'", key))
}

fn string_array(value: &Value, what: &str) -> Result<Vec<String>, String> {
    // A single name is accepted where a list is expected — models write both.
    if let Some(s) = value.as_str() {
        return Ok(vec![s.to_string()]);
    }
    let array = value
        .as_array()
        .ok_or_else(|| format!("'{}' must be a column name or a list of them", what))?;
    array
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("'{}' entries must be strings", what))
        })
        .collect()
}

fn parse_predicates(value: &Value) -> Result<Vec<Predicate>, String> {
    // A lone predicate object is accepted as a one-element list.
    let items: Vec<&Value> = match value {
        Value::Array(a) => a.iter().collect(),
        Value::Object(_) => vec![value],
        _ => return Err("'filter' takes a predicate or a list of them".to_string()),
    };

    items
        .into_iter()
        .map(|item| {
            Ok(Predicate {
                col: required_str(item, "col")?,
                op: PredOp::parse(&required_str(item, "op")?)?,
                value: item.get("value").cloned().unwrap_or(Value::Null),
            })
        })
        .collect()
}

fn parse_aggs(value: Option<&Value>) -> Result<Vec<AggSpec>, String> {
    let value = match value {
        Some(Value::Null) | None => return Ok(Vec::new()),
        Some(v) => v,
    };
    let array = value
        .as_array()
        .ok_or_else(|| "'agg' must be a list of {col, fn} objects".to_string())?;

    array
        .iter()
        .map(|item| {
            let col = required_str(item, "col")?;
            let name = required_str(item, "fn")?;
            let kind = AggregatorKind::all()
                .iter()
                .find(|k| k.name() == name)
                .copied()
                .ok_or_else(|| {
                    format!(
                        "Unknown aggregate '{}'. Available: count, distinct, sum, avg, \
                         min, max, median, stdev, p5, p25, p50, p75, p95",
                        name
                    )
                })?;
            Ok(AggSpec { col, kind })
        })
        .collect()
}

fn parse_join(body: &Value) -> Result<JoinSpec, String> {
    let source_value = body
        .get("source")
        .or_else(|| body.get("path"))
        .ok_or_else(|| "'join' requires a 'source'".to_string())?;
    let source = super::source::Source::from_json(source_value)?;

    let left_on = string_array(body.get("left_on").unwrap_or(&Value::Null), "left_on")?;
    // A single `on` covers the common case of identically named keys.
    let right_on = match body.get("right_on") {
        Some(v) if !v.is_null() => string_array(v, "right_on")?,
        _ => left_on.clone(),
    };

    if left_on.len() != right_on.len() {
        return Err(format!(
            "join needs the same number of keys on both sides, got {} and {}",
            left_on.len(),
            right_on.len()
        ));
    }
    if left_on.is_empty() {
        return Err("'join' requires at least one key in 'left_on'".to_string());
    }

    let how = match body.get("how").and_then(Value::as_str).unwrap_or("inner") {
        "inner" => JoinType::Inner,
        "left" => JoinType::Left,
        "right" => JoinType::Right,
        "outer" | "full" => JoinType::Outer,
        other => {
            return Err(format!(
                "Unknown join type '{}'. Available: inner, left, right, outer",
                other
            ))
        }
    };

    Ok(JoinSpec {
        source,
        left_on,
        right_on,
        how,
    })
}

// ── Application ─────────────────────────────────────────────────────────────

pub fn apply_all(mut df: DataFrame, ops: &[Op]) -> Result<DataFrame, String> {
    for (i, op) in ops.iter().enumerate() {
        df = apply(df, op).map_err(|e| format!("ops[{}]: {}", i, e))?;
    }
    Ok(df)
}

fn apply(mut df: DataFrame, op: &Op) -> Result<DataFrame, String> {
    match op {
        Op::Filter(predicates) => {
            let keep = matching_rows(&df, predicates)?;
            df.row_order = Arc::new(keep);
            df.aggregates_cache = None;
            Ok(df)
        }
        Op::Limit(n) => {
            let mut order = (*df.row_order).clone();
            order.truncate(*n);
            df.row_order = Arc::new(order);
            df.aggregates_cache = None;
            Ok(df)
        }
        Op::Sort { col, desc } => {
            let idx = column_index(&df, col)?;
            df.sort_by(idx, *desc);
            Ok(df)
        }
        Op::Select(names) => select(df, names),
        Op::Compute { name, expr } => {
            let parsed =
                TuiExpr::parse(expr).map_err(|e| format!("in expression '{}': {}", expr, e))?;
            let last = df.columns.len().saturating_sub(1);
            df.add_computed_column(name, &parsed, last)?;
            Ok(df)
        }
        Op::GroupBy { by, agg } => group_by(&df, by, agg),
        Op::Frequency { by, agg } => frequency(&df, by, agg),
        Op::Pivot { index, on, formula } => pivot(&df, index, on, formula),
        Op::Join(spec) => {
            let right = super::source::load_once(&spec.source)?;
            join_dataframes(&df, &right, &spec.left_on, &spec.right_on, spec.how)
                .map_err(|e| e.to_string())
        }
    }
}

/// Index of a column by name, with the available names in the error — a model
/// that guessed wrong can fix it without another round trip.
fn column_index(df: &DataFrame, name: &str) -> Result<usize, String> {
    df.columns
        .iter()
        .position(|c| c.name == name)
        .ok_or_else(|| {
            let available: Vec<&str> = df.columns.iter().map(|c| c.name.as_str()).collect();
            format!(
                "No column named '{}'. Available: {}",
                name,
                available.join(", ")
            )
        })
}

/// Build a `DataFrame` from a Polars frame plus column metadata, the way the
/// TUI does when it opens a derived sheet.
fn from_parts(pdf: polars::prelude::DataFrame, mut columns: Vec<ColumnMeta>) -> DataFrame {
    // An aggregate column inherits the source column's type — `preserves_col_type`
    // is written for the TUI, where the average of an Integer column is still
    // shown with that column's formatting.  Reported to a model, that label is a
    // lie: avg over integers comes back fractional.  The frame knows better.
    for (meta, column) in columns.iter_mut().zip(pdf.columns()) {
        if meta.col_type == ColumnType::Integer
            && matches!(column.dtype(), DataType::Float32 | DataType::Float64)
        {
            meta.col_type = ColumnType::Float;
        }
    }

    let order: Vec<usize> = (0..pdf.height()).collect();
    DataFrame {
        df: pdf,
        columns,
        row_order: Arc::new(order.clone()),
        original_order: Arc::new(order),
        selected_rows: HashSet::new(),
        modified: false,
        aggregates_cache: None,
    }
}

// ── filter ──────────────────────────────────────────────────────────────────

/// Display-row indices surviving every predicate.
///
// ponytail: predicates are joined with AND only. For OR, add an `any_of` key
// alongside `filter` rather than extending crate::data::expression::Expr — its
// Op enum (expression.rs:77) has no And/Or/Not by design.
fn matching_rows(df: &DataFrame, predicates: &[Predicate]) -> Result<Vec<usize>, String> {
    if predicates.is_empty() {
        return Ok((*df.row_order).clone());
    }

    let visible = df.get_visible_df()?;
    let mut keep = vec![true; visible.height()];

    // `contains` runs through DataFrame::find_matching_rows rather than the lazy
    // expression namespace: the lazy `str().contains` needs polars' `regex`
    // feature, which this build does not enable, and find_matching_rows is the
    // same regex over the same rows with tests already on it.
    let mut lazy: Option<Expr> = None;
    for predicate in predicates {
        let idx = column_index(df, &predicate.col)?;

        if predicate.op == PredOp::Contains {
            let pattern = predicate
                .value
                .as_str()
                .ok_or_else(|| "'contains' takes a regex string".to_string())?;
            let mut hit = vec![false; visible.height()];
            for i in df.find_matching_rows(idx, pattern) {
                if i < hit.len() {
                    hit[i] = true;
                }
            }
            for (k, h) in keep.iter_mut().zip(hit) {
                *k &= h;
            }
            continue;
        }

        let dtype = visible
            .column(&predicate.col)
            .map_err(|e| e.to_string())?
            .dtype()
            .clone();
        let expr = predicate_expr(&dtype, predicate)?;
        lazy = Some(match lazy {
            Some(acc) => acc.and(expr),
            None => expr,
        });
    }

    if let Some(expr) = lazy {
        let mask_df = visible
            .lazy()
            .select([expr.alias("__keep")])
            .collect()
            .map_err(|e| format!("filter failed: {}", e))?;

        let mask = mask_df
            .column("__keep")
            .map_err(|e| e.to_string())?
            .as_materialized_series()
            .bool()
            .map_err(|e| format!("filter did not produce a boolean: {}", e))?
            .clone();

        for (k, hit) in keep.iter_mut().zip(&mask) {
            *k &= hit.unwrap_or(false);
        }
    }

    Ok(keep
        .into_iter()
        .enumerate()
        .filter(|(_, k)| *k)
        .map(|(i, _)| df.row_order[i])
        .collect())
}

/// Whether a comparison against this dtype should happen on the text form.
///
/// Dates and datetimes compare as ISO-8601 text, where lexicographic order is
/// chronological order — cheaper than parsing the literal into a temporal type
/// and correct for the formats tuitab produces.
fn compares_as_text(dtype: &DataType) -> bool {
    matches!(
        dtype,
        DataType::String | DataType::Date | DataType::Datetime(_, _) | DataType::Time
    )
}

fn scalar_literal(dtype: &DataType, value: &Value) -> Result<Expr, String> {
    if compares_as_text(dtype) {
        return Ok(lit(json_to_text(value)));
    }
    match value {
        Value::Number(n) if n.is_i64() => Ok(lit(n.as_i64().unwrap())),
        Value::Number(n) => Ok(lit(n.as_f64().unwrap_or(f64::NAN))),
        Value::Bool(b) => Ok(lit(*b)),
        // A numeric column compared against "1000" is a common model slip; take
        // the number rather than failing on the quotes.
        Value::String(s) => s
            .parse::<f64>()
            .map(lit)
            .map_err(|_| format!("'{}' is not a number, but the column is numeric", s)),
        Value::Null => Err("comparison value is null; use is_empty instead".to_string()),
        other => Err(format!("cannot compare against {}", other)),
    }
}

fn json_to_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn predicate_expr(dtype: &DataType, predicate: &Predicate) -> Result<Expr, String> {
    let name = predicate.col.as_str();
    let column = if compares_as_text(dtype) {
        col(name).cast(DataType::String)
    } else {
        col(name)
    };

    Ok(match predicate.op {
        PredOp::IsEmpty => col(name)
            .cast(DataType::String)
            .is_null()
            .or(col(name).cast(DataType::String).eq(lit(""))),
        PredOp::NotEmpty => col(name)
            .cast(DataType::String)
            .is_null()
            .or(col(name).cast(DataType::String).eq(lit("")))
            .not(),
        // Handled eagerly in matching_rows, never reached here.
        PredOp::Contains => return Err("internal: contains is handled separately".to_string()),
        PredOp::Between => {
            let bounds = predicate
                .value
                .as_array()
                .filter(|a| a.len() == 2)
                .ok_or_else(|| "'between' takes a two-element [low, high] array".to_string())?;
            column
                .clone()
                .gt_eq(scalar_literal(dtype, &bounds[0])?)
                .and(column.lt_eq(scalar_literal(dtype, &bounds[1])?))
        }
        PredOp::In | PredOp::NotIn => {
            let items = predicate
                .value
                .as_array()
                .ok_or_else(|| "'in' takes an array of values".to_string())?;
            let mut any: Option<Expr> = None;
            for item in items {
                let eq = column.clone().eq(scalar_literal(dtype, item)?);
                any = Some(match any {
                    Some(acc) => acc.or(eq),
                    None => eq,
                });
            }
            // An empty list matches nothing, which `not` then turns into
            // everything — both are the mathematically right answers.
            let any = any.unwrap_or_else(|| lit(false));
            if predicate.op == PredOp::In {
                any
            } else {
                any.not()
            }
        }
        PredOp::Eq => column.eq(scalar_literal(dtype, &predicate.value)?),
        PredOp::Ne => column.neq(scalar_literal(dtype, &predicate.value)?),
        PredOp::Gt => column.gt(scalar_literal(dtype, &predicate.value)?),
        PredOp::Ge => column.gt_eq(scalar_literal(dtype, &predicate.value)?),
        PredOp::Lt => column.lt(scalar_literal(dtype, &predicate.value)?),
        PredOp::Le => column.lt_eq(scalar_literal(dtype, &predicate.value)?),
    })
}

// ── select ──────────────────────────────────────────────────────────────────

fn select(df: DataFrame, names: &[String]) -> Result<DataFrame, String> {
    let mut metas = Vec::with_capacity(names.len());
    for name in names {
        metas.push(df.columns[column_index(&df, name)?].clone());
    }

    let pdf = df
        .df
        .select(names.iter().map(|s| s.as_str()))
        .map_err(|e| e.to_string())?;

    // Row count is unchanged, so the existing row_order stays valid.
    Ok(DataFrame {
        df: pdf,
        columns: metas,
        ..df
    })
}

// ── group_by ────────────────────────────────────────────────────────────────

/// A plain group-by: exactly the requested aggregates, in the requested order.
///
/// Deliberately not [`DataFrame::build_frequency_table`], which hard-codes a
/// `Count` column and a count-descending sort (`dataframe.rs:1434`, `:1469`).
/// The aggregate semantics still come from tuitab — [`AggregatorKind::to_expr`]
/// is what defines its median, its p95 and its sample stdev.
fn group_by(df: &DataFrame, by: &[String], agg: &[AggSpec]) -> Result<DataFrame, String> {
    if by.is_empty() {
        return Err("'group_by' requires at least one column in 'by'".to_string());
    }

    let visible = df.get_visible_df()?;
    let mut exprs = Vec::with_capacity(agg.len());
    let mut metas: Vec<ColumnMeta> = Vec::new();

    for name in by {
        metas.push(df.columns[column_index(df, name)?].clone());
    }

    for spec in agg {
        // `count` over `*` is a row count, which needs no source column.
        if spec.col == "*" {
            if spec.kind != AggregatorKind::Count {
                return Err(format!(
                    "'{}' needs a column; only 'count' works with '*'",
                    spec.kind.name()
                ));
            }
            exprs.push(len().alias("count"));
            let mut meta = ColumnMeta::new("count".to_string());
            meta.col_type = ColumnType::Integer;
            metas.push(meta);
            continue;
        }

        let source = &df.columns[column_index(df, &spec.col)?];

        // build_frequency_table skips an incompatible aggregator silently
        // (dataframe.rs:1445). Here that would hand the model a short answer it
        // has no way to notice, so it is an error instead.
        if !spec.kind.is_compatible(source.col_type) {
            return Err(format!(
                "Cannot compute {} over '{}': the column is {}, and {} needs a numeric one",
                spec.kind.name(),
                spec.col,
                super::render::type_name(source.col_type),
                spec.kind.name()
            ));
        }

        let expr = spec.kind.to_expr(&spec.col).ok_or_else(|| {
            format!(
                "'{}' is not available as a group aggregate",
                spec.kind.name()
            )
        })?;

        let alias = format!("{}:{}", spec.col, spec.kind.name());
        exprs.push(expr.alias(&alias));

        let mut meta = ColumnMeta::new(alias);
        if spec.kind.preserves_col_type() {
            meta.col_type = source.col_type;
            meta.currency = source.currency;
            meta.precision = source.precision;
        } else {
            meta.col_type = ColumnType::Integer;
        }
        metas.push(meta);
    }

    if exprs.is_empty() {
        return Err("'group_by' requires at least one aggregate in 'agg'".to_string());
    }

    // `group_by_stable`, not `group_by`: the unstable variant returns groups in
    // an arbitrary order, and a tool sold on determinism cannot do that.
    let grouped = visible
        .lazy()
        .group_by_stable(by.iter().map(|s| col(s.as_str())).collect::<Vec<_>>())
        .agg(exprs)
        .collect()
        .map_err(|e| format!("group_by failed: {}", e))?;

    Ok(from_parts(grouped, metas))
}

// ── frequency ───────────────────────────────────────────────────────────────

/// Count-ranked distribution, with a `Pct` share column.
///
/// This is [`DataFrame::build_frequency_table`] as it actually behaves: `Count`
/// is always present and the result is sorted by it descending.  The `Bar`
/// column it appends is an ASCII sparkline for the TUI and pure noise in JSON,
/// so it is dropped here rather than by changing a function the TUI depends on.
fn frequency(df: &DataFrame, by: &[String], agg: &[AggSpec]) -> Result<DataFrame, String> {
    if by.is_empty() {
        return Err("'frequency' requires at least one column in 'by'".to_string());
    }

    let group_indices: Vec<usize> = by
        .iter()
        .map(|name| column_index(df, name))
        .collect::<Result<_, _>>()?;

    let mut aggregated: Vec<(usize, Vec<AggregatorKind>)> = Vec::new();
    for spec in agg {
        let idx = column_index(df, &spec.col)?;
        let source = &df.columns[idx];
        if !spec.kind.is_compatible(source.col_type) {
            return Err(format!(
                "Cannot compute {} over '{}': the column is {}, and {} needs a numeric one",
                spec.kind.name(),
                spec.col,
                super::render::type_name(source.col_type),
                spec.kind.name()
            ));
        }
        match aggregated.iter_mut().find(|(i, _)| *i == idx) {
            Some((_, kinds)) => kinds.push(spec.kind),
            None => aggregated.push((idx, vec![spec.kind])),
        }
    }

    let (pdf, metas) = if group_indices.len() == 1 {
        df.build_frequency_table(group_indices[0], &aggregated)?
    } else {
        df.build_multi_frequency_table(&group_indices, &aggregated)?
    };

    let mut result = from_parts(pdf, metas);
    if let Some(bar) = result.columns.iter().position(|c| c.name == "Bar") {
        result.drop_column(bar)?;
    }
    Ok(result)
}

// ── pivot ───────────────────────────────────────────────────────────────────

fn pivot(df: &DataFrame, index: &[String], on: &str, formula: &str) -> Result<DataFrame, String> {
    if index.is_empty() {
        return Err("'pivot' requires at least one column in 'index'".to_string());
    }
    for name in index {
        column_index(df, name)?;
    }
    column_index(df, on)?;

    let expr =
        TuiExpr::parse(formula).map_err(|e| format!("in pivot formula '{}': {}", formula, e))?;

    let (pdf, metas) = df.create_pivot_table(index, on, &expr)?;
    Ok(from_parts(pdf, metas))
}
