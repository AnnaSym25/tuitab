use crate::data::dataframe::DataFrame;
use polars::prelude::*;

impl DataFrame {
    /// Sort visible rows by column `col_idx` using Polars native `arg_sort`.
    ///
    /// Builds a sub-DataFrame of visible rows via `get_visible_df()`, runs
    /// `Series::arg_sort()` (SIMD / radix-sort on native types — no String
    /// conversion), then maps the resulting indices back to physical row
    /// positions and updates `row_order`.
    pub fn sort_by(&mut self, col_idx: usize, descending: bool) {
        if col_idx >= self.df.width() || col_idx >= self.columns.len() {
            return;
        }

        let col_name = self.columns[col_idx].name.clone();

        // Build a sub-DataFrame containing only the visible rows
        let visible = match self.get_visible_df() {
            Ok(df) => df,
            Err(_) => return,
        };

        // Retrieve column and run native arg_sort
        let series = match visible.column(&col_name) {
            Ok(c) => c.as_materialized_series().clone(),
            Err(_) => return,
        };

        let sort_options = SortOptions {
            descending,
            nulls_last: true,
            ..Default::default()
        };

        let sorted_idx = series.arg_sort(sort_options);

        // Map arg_sort indices (positions inside `visible`) back to physical
        // row indices (positions inside the original `self.df`).
        let new_order: Vec<usize> = sorted_idx
            .into_no_null_iter()
            .map(|i| self.row_order[i as usize])
            .collect();

        self.row_order = std::sync::Arc::new(new_order);
        self.aggregates_cache = None;
    }

    /// Sort visible rows by several keys at once, the first key most significant.
    ///
    /// Not the same as calling [`DataFrame::sort_by`] twice.  A single-key sort
    /// leaves `maintain_order` at its default of `false`, so a second sort is
    /// free to shuffle rows that tie on its own key — losing whatever order the
    /// first one established.  Chaining looks like a compound sort and is not
    /// one; this is.
    ///
    /// Works by carrying a position column through the sort and reading the
    /// permutation back out of it, which keeps the mapping to physical rows
    /// exact without asking Polars for one.
    pub fn sort_by_keys(&mut self, keys: &[(usize, bool)]) -> Result<(), String> {
        if keys.is_empty() {
            return Err("sorting needs at least one key".to_string());
        }
        if let Some((idx, _)) = keys
            .iter()
            .find(|(idx, _)| *idx >= self.df.width() || *idx >= self.columns.len())
        {
            return Err(format!(
                "no column at index {}; the table has {}",
                idx,
                self.columns.len()
            ));
        }

        let visible = self.get_visible_df()?;
        let height = visible.height();

        // A name no real column can shadow.
        let mut marker = "__tuitab_pos".to_string();
        while visible.column(&marker).is_ok() {
            marker.push('_');
        }

        let mut staged = visible;
        let positions: Vec<u32> = (0..height as u32).collect();
        staged
            .with_column(Series::new(marker.as_str().into(), positions).into())
            .map_err(|e| e.to_string())?;

        let names: Vec<String> = keys
            .iter()
            .map(|(idx, _)| self.columns[*idx].name.clone())
            .collect();
        let descending: Vec<bool> = keys.iter().map(|(_, desc)| *desc).collect();

        let sorted = staged
            .sort(
                names,
                SortMultipleOptions::new()
                    .with_order_descending_multi(descending)
                    .with_nulls_last(true)
                    .with_maintain_order(true),
            )
            .map_err(|e| format!("sort failed: {}", e))?;

        let permutation = sorted
            .column(&marker)
            .and_then(|c| c.u32().cloned())
            .map_err(|e| e.to_string())?;

        let new_order: Vec<usize> = permutation
            .into_no_null_iter()
            .map(|i| self.row_order[i as usize])
            .collect();

        self.row_order = std::sync::Arc::new(new_order);
        self.aggregates_cache = None;
        Ok(())
    }

    /// Reset row_order to the original load order.
    pub fn reset_sort(&mut self) {
        self.row_order = self.original_order.clone();
        self.aggregates_cache = None;
    }
}
