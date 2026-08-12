use crate::data::aggregator::AggregatorKind;
use crate::data::expression::Expr;
use crate::types::{ColumnType, CurrencyKind};
use serde::{Deserialize, Serialize};

/// Two-state toggle for column display width (`_` key).
/// Header width (name + 2 chars padding) is the floor in both modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ColumnWidthMode {
    /// Width auto-calculated at load time (bounded, ~40 chars max, samples first 1000 rows).
    #[default]
    Default,
    /// Fitted to full content width across all rows. Never less than the header width.
    Fit,
}

/// Metadata about a single data column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMeta {
    /// Column name (from CSV header, or auto-generated)
    pub name: String,
    /// Inferred (or user-assigned) type of the column data
    pub col_type: ColumnType,
    /// Display width in characters (auto-calculated)
    pub width: u16,
    /// Minimum width (length of column name + 2 for type icon)
    pub min_width: u16,
    /// Active aggregators assigned by the user (Phase 12)
    pub aggregators: Vec<AggregatorKind>,
    /// Expression for computed columns (None for regular data columns)
    pub expression: Option<Expr>,
    /// Number of decimal places to display for numeric types
    pub precision: u8,
    /// Whether this column is pinned to the left
    pub pinned: bool,
    /// Currency kind, used when col_type == Currency
    pub currency: Option<CurrencyKind>,
    /// Current width display mode (Default / Fit).
    /// Old sessions that lack this field get Default.
    #[serde(default)]
    pub width_mode: ColumnWidthMode,
    /// Width saved the first time calc_column_width runs (= load-time width).
    /// Used by the Default mode to restore the original auto-calculated width.
    #[serde(default)]
    pub default_width: u16,
    /// Whether this column is selected (zs/zu in z-prefix mode)
    pub selected: bool,
    /// Backup of original Datetime values before converting to Date
    /// Stores formatted datetime strings for recovery
    pub backup_datetime_str: Option<Vec<Option<String>>>,

    /// The name this column had in the database table the sheet was loaded from.
    ///
    /// `None` means it was not there — created by `zi`, `=` or `zx` — or that the sheet
    /// did not come from a database at all.  Renaming changes `name` and leaves this
    /// alone, which is what makes A→B→C a single `RENAME COLUMN` rather than two, and
    /// what tells an added column apart from a renamed one after the fact.
    #[serde(default)]
    pub db_origin: Option<String>,

    /// The type the user deliberately assigned with `t`.
    ///
    /// Separate from [`Self::col_type`] because `col_replace` (`zr`/`zg`) force-sets
    /// `String` as a side effect of a find-and-replace, and a find-and-replace must
    /// never turn into an `ALTER COLUMN … TYPE`.
    #[serde(default)]
    pub db_retype: Option<ColumnType>,
}

impl ColumnMeta {
    pub fn new(name: String) -> Self {
        let name_w = unicode_width::UnicodeWidthStr::width(name.as_str()) as u16;
        // +2: 1 separator space + 1 char for the type icon so the icon never covers the name
        let min_width = name_w + 2;
        Self {
            name,
            col_type: ColumnType::String,
            min_width,
            width: min_width.max(8), // default minimum 8 chars
            aggregators: Vec::new(),
            expression: None,
            precision: 2,
            pinned: false,
            currency: None,
            width_mode: ColumnWidthMode::Default,
            default_width: 0,
            selected: false,
            backup_datetime_str: None,
            db_origin: None,
            db_retype: None,
        }
    }
}
