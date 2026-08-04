//! Finding and removing duplicate rows.
//!
//! Rows are grouped by the **stored** value of their key columns, rendered as
//! text: `1` and `1.0` in a column read as text are different keys, while the
//! same number in a numeric column is one key however it was written in the
//! file.
//!
//! Deliberately not the value as displayed. A column shown with two decimals
//! renders 1.504 and 1.496 identically, and calling those duplicates would
//! discard one of two different numbers — a rounding rule chosen for reading is
//! no basis for deciding what data to drop.

use crate::data::dataframe::DataFrame;
use indexmap::IndexMap;
use std::collections::HashSet;

/// Which row of a duplicate group to keep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    /// The one that appears first in the current row order.
    First,
    /// The one that appears last.
    Last,
    /// The one with the smallest value in a column.
    Min(usize),
    /// The one with the largest value in a column.
    Max(usize),
    /// One at random.
    ///
    /// Carries the seed so the choice can be repeated: an unseeded caller gets
    /// a seed generated for it, and is told which one, rather than an answer
    /// nobody can reproduce.
    Random(u64),
}

impl Keep {
    /// Parse the name a caller uses, pairing `random` with a seed.
    pub fn parse(name: &str, seed: Option<u64>, column: Option<usize>) -> Result<Self, String> {
        Ok(match name {
            "first" => Self::First,
            "last" => Self::Last,
            "min" => Self::Min(column.ok_or("'min' needs a column to compare")?),
            "max" => Self::Max(column.ok_or("'max' needs a column to compare")?),
            "random" => Self::Random(seed.unwrap_or_else(random_seed)),
            other => {
                return Err(format!(
                    "Unknown keep rule '{}'. Available: first, last, min, max, random",
                    other
                ))
            }
        })
    }
}

/// A seed drawn from the system source, for callers that did not supply one.
pub fn random_seed() -> u64 {
    rand::random()
}

/// Compare two cells the way a person reading them would: numerically when
/// both parse as numbers, and as text otherwise.
///
/// Without the numeric branch `"10"` would sort before `"9"`.
fn compare_cells(a: &str, b: &str) -> std::cmp::Ordering {
    if let (Ok(x), Ok(y)) = (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
        return x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal);
    }
    a.cmp(b)
}

/// Group the visible rows by the values in `key_cols`.
///
/// Insertion-ordered, so every caller downstream is deterministic without
/// having to sort afterwards.
fn group_rows(df: &DataFrame, key_cols: &[usize]) -> IndexMap<Vec<String>, Vec<usize>> {
    let mut groups: IndexMap<Vec<String>, Vec<usize>> = IndexMap::new();
    for &physical in df.row_order.iter() {
        let key: Vec<String> = key_cols
            .iter()
            .map(|&c| df.get_physical(physical, c))
            .collect();
        groups.entry(key).or_default().push(physical);
    }
    groups
}

/// The physical rows that appear more than once, keyed on `key_cols`.
///
/// An empty `key_cols` compares whole rows, which is what "find duplicates"
/// means with nothing selected.
pub fn duplicate_rows(df: &DataFrame, key_cols: &[usize]) -> Vec<usize> {
    let all: Vec<usize>;
    let keys = if key_cols.is_empty() {
        all = (0..df.columns.len()).collect();
        &all
    } else {
        key_cols
    };

    group_rows(df, keys)
        .into_values()
        .filter(|rows| rows.len() > 1)
        .flatten()
        .collect()
}

/// Keep one row per group, dropping the rest.
///
/// Returns the surviving physical rows in their original relative order — the
/// caller decides whether that becomes a new `row_order` or a new frame.
pub fn deduplicate(df: &DataFrame, key_cols: &[usize], keep: Keep) -> Result<Vec<usize>, String> {
    if key_cols.is_empty() {
        return Err("deduplication needs at least one column to compare".to_string());
    }
    for &c in key_cols {
        if c >= df.columns.len() {
            return Err(format!("column index {} is out of range", c));
        }
    }
    if let Keep::Min(c) | Keep::Max(c) = keep {
        if c >= df.columns.len() {
            return Err(format!("tiebreaker column index {} is out of range", c));
        }
    }

    let groups = group_rows(df, key_cols);

    let keepers: HashSet<usize> = match keep {
        Keep::First => groups
            .values()
            .filter_map(|rows| rows.first().copied())
            .collect(),
        Keep::Last => groups
            .values()
            .filter_map(|rows| rows.last().copied())
            .collect(),
        Keep::Random(seed) => {
            use rand::seq::IndexedRandom;
            use rand::SeedableRng;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            groups
                .values()
                .filter_map(|rows| rows.choose(&mut rng).copied())
                .collect()
        }
        Keep::Min(col) | Keep::Max(col) => {
            let want_greater = matches!(keep, Keep::Max(_));
            groups
                .values()
                .filter_map(|rows| {
                    let mut best = *rows.first()?;
                    let mut best_value = df.get_physical(best, col);
                    for &physical in rows.iter().skip(1) {
                        let value = df.get_physical(physical, col);
                        let ordering = compare_cells(&value, &best_value);
                        let better = if want_greater {
                            ordering == std::cmp::Ordering::Greater
                        } else {
                            ordering == std::cmp::Ordering::Less
                        };
                        if better {
                            best = physical;
                            best_value = value;
                        }
                    }
                    Some(best)
                })
                .collect()
        }
    };

    Ok(df
        .row_order
        .iter()
        .copied()
        .filter(|r| keepers.contains(r))
        .collect())
}

// ── sampling ────────────────────────────────────────────────────────────────

/// Pick `n` of the visible rows at random, returned in their original order.
///
/// Fewer than `n` visible rows returns all of them rather than failing: asking
/// for a hundred out of twenty is a reasonable thing to do.
///
/// The seed is a parameter rather than drawn inside, because a sample nobody
/// can reproduce is a poor answer to give a caller that will quote it. Use
/// [`random_seed`] when you do not care, and report the value you used.
pub fn sample_rows(df: &DataFrame, n: usize, seed: u64) -> Vec<usize> {
    use rand::SeedableRng;

    let visible = df.row_order.len();
    let take = n.min(visible);

    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    // Positions are drawn, not values. Drawing values meant restoring the
    // table's order with `row_order.iter().position(…)` as the sort key — a
    // linear scan for every comparison, so a thousand rows out of a million was
    // on the order of 10^10 operations. Positions sort numerically, and mapping
    // them back to physical rows afterwards is one pass.
    let mut chosen: Vec<usize> = rand::seq::index::sample(&mut rng, visible, take).into_vec();
    chosen.sort_unstable();
    chosen.into_iter().map(|i| df.row_order[i]).collect()
}
