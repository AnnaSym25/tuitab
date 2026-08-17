//! Checking one typed-in value against a column's [`ColumnType`], before it is stored.
//!
//! Everywhere else in the codebase a value meets its type too late to help anyone:
//! [`crate::data::dataframe::DataFrame::set_cell`] falls back to `unwrap_or(new_series)`,
//! so `abc` typed into an Int64 column silently turns the whole column into text, and
//! the complaint only arrives at save time, and only for a database. The row form asks
//! the question while the user is still looking at the field.
//!
//! What counts as valid is taken from the parsers `set_column_type` already uses
//! (`col_bool_from_str` and friends), so the form accepts what the `t` key accepts.
//! One deliberate divergence: `Boolean` here is strict. `col_bool_from_str` maps
//! anything that is not `true`/`1`/`yes` to `false`, which is a reasonable rule for
//! converting a whole column and a bad one for a form whose entire job is to catch
//! the typo.
//!
//! The parsed value comes back as a primitive rather than as canonical text on
//! purpose: a string is not a route into a Date or Datetime column here — that is
//! why `col_date_from_str` computes days-since-epoch itself instead of casting text.

use chrono::{NaiveDate, NaiveDateTime};

use crate::types::ColumnType;

/// One parsed cell, in the shape the column's series is built from.
#[derive(Debug, Clone, PartialEq)]
pub enum TypedCell {
    Str(String),
    I64(i64),
    F64(f64),
    Bool(bool),
    /// Days since 1970-01-01, the representation a Polars `Date` casts from.
    DateDays(i32),
    /// Microseconds since 1970-01-01T00:00:00, for `Datetime(Microseconds, None)`.
    DatetimeMicros(i64),
}

/// Parse `raw` for a column of `col_type`.
///
/// `Ok(None)` is an empty field, which is a NULL rather than an empty string — the
/// same choice [`crate::data::dataframe::DataFrame::insert_empty_row`] makes.
/// `Err` carries a short phrase meant to be shown under the field.
pub fn parse_typed_value(raw: &str, col_type: ColumnType) -> Result<Option<TypedCell>, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Ok(None);
    }

    let cell = match col_type {
        ColumnType::String => TypedCell::Str(raw.to_string()),

        ColumnType::Integer | ColumnType::FileSize => TypedCell::I64(
            s.parse::<i64>()
                .map_err(|_| "not a whole number".to_string())?,
        ),

        ColumnType::Float => {
            TypedCell::F64(s.parse::<f64>().map_err(|_| "not a number".to_string())?)
        }

        ColumnType::Boolean => match s.to_lowercase().as_str() {
            "true" | "1" | "yes" | "y" => TypedCell::Bool(true),
            "false" | "0" | "no" | "n" => TypedCell::Bool(false),
            _ => return Err("not true/false (also 1/0, yes/no)".into()),
        },

        // A percentage column holds the fraction and displays the percent, so what is
        // typed is read as a percent — the same direction `parse_typed_input` takes.
        ColumnType::Percentage => {
            let n = s
                .replace('%', "")
                .trim()
                .parse::<f64>()
                .map_err(|_| "not a percentage (e.g. 45 or 45%)".to_string())?;
            TypedCell::F64(n / 100.0)
        }

        // Negative money is displayed in brackets and carries no minus, so the sign
        // has to be read off the brackets before they are filtered out.
        ColumnType::Currency => {
            let bracketed = s.starts_with('(') && s.ends_with(')');
            let cleaned: String = s
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect();
            let n = cleaned
                .parse::<f64>()
                .map_err(|_| "not an amount (e.g. 1200.50)".to_string())?;
            TypedCell::F64(if bracketed { -n.abs() } else { n })
        }

        ColumnType::Date => {
            TypedCell::DateDays(parse_date_days(s).ok_or("not a date (YYYY-MM-DD)")?)
        }

        ColumnType::Datetime => TypedCell::DatetimeMicros(
            parse_datetime_micros(s).ok_or("not a date and time (YYYY-MM-DD HH:MM:SS)")?,
        ),
    };
    Ok(Some(cell))
}

/// Days since the epoch, trying the same formats as `DataFrame::col_date_from_str`.
fn parse_date_days(s: &str) -> Option<i32> {
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1)?;
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some((d - epoch).num_days() as i32);
    }
    for fmt in crate::data::DATETIME_FORMATS {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some((dt.date() - epoch).num_days() as i32);
        }
    }
    None
}

/// Microseconds since the epoch, matching `DataFrame::col_datetime_from_str`.
fn parse_datetime_micros(s: &str) -> Option<i64> {
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1)?.and_hms_opt(0, 0, 0)?;
    for fmt in crate::data::DATETIME_FORMATS {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            let diff = dt - epoch;
            return diff
                .num_microseconds()
                .or_else(|| diff.num_seconds().checked_mul(1_000_000));
        }
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let diff = d.and_hms_opt(0, 0, 0)? - epoch;
        return diff
            .num_microseconds()
            .or_else(|| diff.num_seconds().checked_mul(1_000_000));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_null_for_every_type() {
        for t in ColumnType::all() {
            assert_eq!(parse_typed_value("   ", *t), Ok(None), "{:?}", t);
        }
    }

    #[test]
    fn every_type_accepts_its_own_and_rejects_the_rest() {
        let cases: &[(ColumnType, &str, TypedCell, &str)] = &[
            (
                ColumnType::String,
                "abc",
                TypedCell::Str("abc".into()),
                // Nothing is invalid for a string column, so this one is checked below.
                "",
            ),
            (ColumnType::Integer, "42", TypedCell::I64(42), "4.5"),
            (ColumnType::FileSize, "1024", TypedCell::I64(1024), "1.5 MB"),
            (ColumnType::Float, "4.5", TypedCell::F64(4.5), "abc"),
            (ColumnType::Boolean, "yes", TypedCell::Bool(true), "abc"),
            (
                ColumnType::Percentage,
                "45%",
                TypedCell::F64(0.45),
                "half of it",
            ),
            (
                ColumnType::Currency,
                "($5.00)",
                TypedCell::F64(-5.0),
                "free",
            ),
            (
                ColumnType::Date,
                "2024-01-05",
                TypedCell::DateDays(19727),
                "2024-13-01",
            ),
            (
                ColumnType::Datetime,
                "2024-01-05 10:30:00",
                TypedCell::DatetimeMicros(19727 * 86_400_000_000 + 37_800_000_000),
                "yesterday",
            ),
        ];

        for (t, good, want, bad) in cases {
            assert_eq!(
                parse_typed_value(good, *t),
                Ok(Some(want.clone())),
                "{:?} should accept {:?}",
                t,
                good
            );
            if !bad.is_empty() {
                assert!(
                    parse_typed_value(bad, *t).is_err(),
                    "{:?} should reject {:?}",
                    t,
                    bad
                );
            }
        }
        // A string column takes anything, including what every other type refuses.
        assert!(parse_typed_value("yesterday", ColumnType::String).is_ok());
    }
}
