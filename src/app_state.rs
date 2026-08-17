use crate::data::aggregator::AggregatorKind;
use crate::data::dataframe::DataFrame;
use crate::types::{ChartAgg, CopyPending, JoinContextItem};
use crate::ui::text_input::TextInput;
use std::collections::HashSet;

#[derive(Default)]
pub struct SaveState {
    pub input: TextInput,
    pub error: Option<String>,
    pub autocomplete_prefix: String,
    pub autocomplete_candidates: Vec<String>,
    pub autocomplete_idx: usize,
    /// Shapes offered for the sheet being saved, and the highlighted one.  The cursor
    /// is transient; `shape` is what carries across saves — remembering the *index*
    /// would silently pick a different shape once the option list changes length.
    pub shapes: Vec<crate::data::io::doc_io::Shape>,
    pub shape_index: usize,
    pub shape: crate::data::io::doc_io::Shape,
    /// Target path, held while the shape popup is up.
    pub pending_path: Option<std::path::PathBuf>,
    /// Name being typed for the table a database export will create.
    pub table_input: TextInput,
    /// The answered table name, parked exactly like `pending_path` so re-entering
    /// `ApplySave` does not ask a second time.
    pub table_name: Option<String>,
}

/// The SQL confirmation popup: what is about to run, and where.
#[derive(Default)]
pub struct SqlConfirmState {
    pub plan: Option<crate::data::io::db_write::WritePlan>,
    /// First visible line of the statement list.
    pub scroll: usize,
    /// Largest useful [`Self::scroll`], recorded by the renderer because only it knows
    /// the terminal size and how many lines the statements wrapped to.  One frame
    /// behind after a resize, which no one can type fast enough to notice.
    pub max_scroll: std::cell::Cell<usize>,
    /// The table a create-plan is about to make.  `apply_sql_plan` reads this when the
    /// sheet has no `table_source` of its own — which is the whole point of creating one.
    pub pending_source: Option<crate::data::io::db_write::TableSource>,
    /// Target database, parked while the popup is up.  Deliberately not
    /// [`SaveState::pending_path`]: sharing that field would let a stale value reach the
    /// shape popup.
    pub path: Option<std::path::PathBuf>,
}

#[derive(Default)]
pub struct AggregatorState {
    pub select_index: usize,
    pub selected: HashSet<AggregatorKind>,
}

/// Which window function `zw` is offering.
#[derive(Default)]
pub struct WindowFnState {
    pub select_index: usize,
    /// Which column orders the rows for the window, from the order picker.
    /// `None` is the table's own order.
    pub order_by: Option<String>,
    /// Highlighted row of the order picker: 0 is "the table's order", and
    /// column `i` is at `i + 1`.
    pub order_index: usize,
    /// Set by the direction picker, read by `add_window`. Lives here rather
    /// than on `App` so that resetting it is the same one line that resets
    /// `select_index` when `zw` opens — a second field with its own cleanup on
    /// each cancel path is how `pending_window_fn` came to survive Esc.
    pub desc: bool,
}

#[derive(Default)]
pub struct TypeSelectState {
    pub index: usize,
    pub currency_index: usize,
}

#[derive(Default)]
pub struct PartitionState {
    pub select_index: usize,
    pub selected: HashSet<String>,
}

#[derive(Default)]
pub struct ExpressionState {
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub autocomplete_candidates: Vec<String>,
    pub autocomplete_idx: usize,
    pub autocomplete_prefix: String,
}

#[derive(Default)]
pub struct PivotState {
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
}

pub enum ChartDrillKey {
    Exact(String),
    Range(f64, f64),
}

pub struct ChartState {
    pub ref_col: Option<usize>,
    pub agg: ChartAgg,
    pub agg_index: usize,
    pub cursor_bin: usize,
    pub drill_keys: Vec<ChartDrillKey>,
    /// Set when entering a drill-down table from the chart so pop_sheet can return to chart mode.
    pub drill_return: bool,
}

impl Default for ChartState {
    fn default() -> Self {
        Self {
            ref_col: None,
            agg: ChartAgg::Count,
            agg_index: 0,
            cursor_bin: 0,
            drill_keys: vec![],
            drill_return: false,
        }
    }
}

#[derive(Default)]
pub struct JoinState {
    pub source_index: usize,
    pub other_df: Option<DataFrame>,
    pub other_title: String,
    pub type_index: usize,
    pub left_keys: Vec<String>,
    pub right_keys: Vec<String>,
    pub left_key_index: usize,
    pub right_key_index: usize,
    pub path_input: TextInput,
    pub path_error: Option<String>,
    pub context_items: Vec<JoinContextItem>,
    pub overview_cursor: usize,
    pub overview_selected: Vec<usize>,
    pub pending_queue: Vec<JoinContextItem>,
}

/// The new-row form (`O`): one text field per column, checked as it is typed.
///
/// The fields are kept parallel to the sheet's columns rather than keyed by name —
/// the form is rebuilt from scratch every time it opens, so a rename or an added
/// column in between cannot leave a stale key behind.
#[derive(Default)]
pub struct RowFormState {
    pub fields: Vec<TextInput>,
    /// Why the field at the same index is not acceptable yet, if it is not.
    pub errors: Vec<Option<String>>,
    pub focus: usize,
    /// Enter found empty fields and said so; a second Enter accepts them as NULL.
    ///
    /// Cleared by any other key, so the confirmation cannot survive a change to what
    /// it was about.
    pub confirm_empty: bool,
}

#[derive(Default)]
pub struct CopyState {
    pub pending: Option<CopyPending>,
    pub format_index: usize,
}

/// Tiebreaker selection for smart dedup (Shift+S → D, when pinned cols exist).
///
/// `options[0]` is always `None` — the "Random" choice. The rest are
/// `Some((column_index, descending))` pairs covering every non-pinned column
/// in both ASC and DESC directions.
#[derive(Default)]
pub struct DedupTiebreakerState {
    pub options: Vec<Option<(usize, bool)>>,
    pub select_index: usize,
    /// Pinned columns used as the dedup grouping key.
    pub key_cols: Vec<usize>,
}
