# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0] - 2026-08-04

### Fixed

- **Repeating `z` + arrow moved the column every other time, and moved the
  cursor in between.** The first chord leaves the app in a column-move mode
  where bare arrows keep reordering — but the chord people repeat is the whole
  `z` + arrow, and that `z` fell through to the mode's catch-all and left it, so
  the arrow behind it moved the cursor. The mode has behaved this way since
  before 0.6.0; what made it visible is the compound sort added here, because
  the drifted cursor now had `z[` to land on and the sort went to a column the
  user never pointed at. `z` re-opens the prefix from column-move mode, on
  Cyrillic layouts too — `я`, and `р`/`д` for left and right.
- **The sort arrow never fitted in the header.** Column widths are measured from
  the name alone, so `!`, `*` and `▲` had no room reserved and the whole string
  was truncated from the right — cutting off the arrow, which is last. A
  compound sort made it worse by needing a third column for the rank digit. The
  name now gives way instead, since it is the part with information to spare.
- **A running total could only be ordered by re-sorting the table.** `cum_sum`,
  `lag`, `lead` and `row_number` read the frame as it stood, so totalling by
  date meant sorting by date first — a change to the table nobody asked for, and
  one the documentation used to recommend. `zw` now asks which column orders the
  rows, and `window` takes `order_by`: the rows are put in that order to compute
  the column and the answer comes back where they already are. Ties keep their
  relative order and empty values sort last. The eight functions that read no
  order refuse an `order_by` rather than dropping it, since
  `RANK() OVER (ORDER BY x)` is a reasonable thing to ask for and a bad thing to
  answer differently.
- **A rank in the TUI was always ascending.** The server's `window` operation
  accepts `desc`; `zw` had no way to say it, so one of the two surfaces could
  not ask for a top-N at all. Picking `rank` or
  `dense_rank` now asks which end comes first, before the partition picker.
  Ascending stays the default on both surfaces, and a descending rank is named
  `<col>_rank_desc` so it can sit beside the ascending one.
- **`is_empty` matched nothing, on any file.** It compared the column against a
  null literal, and in three-valued logic `x == null` is null rather than true.
  Since polars reads a blank CSV field as null, a blank cell matched neither
  `is_empty` nor `not_empty` — it fell out of both halves of a partition that is
  supposed to cover every row.
- **Filtering a date column was impossible.** Polars refuses to compare a
  temporal column with a string and nothing cast either side. Dates now compare
  as ISO text, which orders identically to chronology.
- **A quoted number no longer fails against a numeric column.** `"30"` where
  `30` was meant is a slip worth absorbing; `"a lot"` is still a type error.
- **Integer ids above 2^53 stopped being distinguishable.** JSON has one number
  type, so an id arrived as `f64` and promoted the column to float; two
  neighbouring ids then matched the same query. Integral literals stay integral.
- **A whole-frame condition reported "1 row matched".** `mean(age) > 30` lowers
  to a single verdict about the table, which applies to all its rows or none —
  enumerating it answered about row zero and called that the answer.
- **A refused filter said only that it had been refused.** Five different causes
  shared one message; the real reason now reaches the caller.
- **`z[` left the z-prefix open**, so the next key was read as a z-command:
  pressing `d` to delete selected rows deleted a column instead, and a compound
  sort could not be typed at all.
- **Deleting or moving a sorted column corrupted the sort.** Keys were stored as
  column positions, which every column operation renumbers — deleting one left a
  key pointing past the end and the next `z[` panicked. Keys are stored by name
  and resolved when used.
- **`zw` on a JSON/YAML/TOML sheet broke it permanently.** Adding a column
  desynchronised the table from its document, after which editing refused for
  the rest of the session. It now declines, as `zf` already did.
- **A new column taking an existing name desynchronised the table.** Polars
  replaces the old column while the metadata was appended regardless, so a
  reader received one more header than there were values in each row. Both
  `window` and `compute` now refuse the collision.
- **Transposing a table that merely had a pinned column called `column`**
  inverted it instead, dropping a column. Recognising its own output is now a
  flag nothing but the transpose sets, rather than a shape ordinary data can
  have — `build_multi_frequency_table` pins its group columns, so a frequency
  table followed by a transpose was enough to trigger it.
- **A sort polars refused returned quietly**, leaving the rows as they were with
  nothing to distinguish that from an already-sorted table.
- **A projection changed a column's reported type.** `select` built its frame by
  hand and skipped the reconciliation every other derived table gets.
- **Adding a window column threw away the sheet's state.** `r` no longer
  restored the file's own row order, and a selection disappeared under the user,
  because the added column was built as a fresh table rather than as an
  addition.
- **`zw` refused eight of its twelve functions on a text column**, with a
  message about percent columns nobody had asked for — it was being judged by a
  gate written for `zF`. `row_number` reads no column at all. A function that
  does need numbers now names itself when it declines.
- **A window column is inserted beside the column it describes** rather than at
  the far right, where a wide table put it off-screen.
- **A frequency table ordered its groups differently between runs.** Groups came
  back in hash order and ties in count fell where they may. Order is now first
  appearance in the data, which is stable and means something.
- **A group of blank cells was counted as zero rows** and given a zero share,
  because the count skipped missing values while claiming to count rows.
- **A frequency table silently dropped an aggregate over one of its grouping
  columns**, having promised to refuse aggregates it cannot compute.
- **`zF` and `T` corrupted a JSON/YAML/TOML sheet**, exactly as `zw` did: the
  table stopped matching its document, editing refused for the rest of the
  session, and the result vanished at the next reprojection. `zf` had always
  declined; these two now do too.
- **Sorting and every chord were unreachable on a non-Latin layout.** The layout
  translation ran in a fallback at the bottom of Normal mode — a second copy of
  the key bindings that was already missing the brackets, and that never saw the
  second key of a chord at all, so `zw`, `gb`, `z[` and `z]` could not be typed.
  Translation now happens once, before anything reads the key, and the duplicate
  table is gone.
- **Random sampling was quadratic in the size of the table.** Restoring the
  table's order used a linear scan per comparison, so a thousand rows out of a
  million was on the order of 10^10 operations. Positions are drawn and sorted
  instead.
- **An aggregate in an MCP `compute` ignored the preceding `filter`.** A pipeline
  that narrowed the rows and then asked for `amount / sum(amount)` divided by the
  total of every row in the file, dropped ones included — a share-of-total that
  looked entirely reasonable and was not. Every pipeline operation now hands the
  next one a materialised frame, so nothing downstream can reach past the view to
  the data underneath.
- **A column whose name looks like a regular expression addressed the wrong
  columns — or none.** Polars reads a name that starts with `^` and ends with `$`
  as a pattern selecting every column it matches, and tuitab passed header names
  straight through. A file with a column literally called `^total$` turned a
  frequency table into "unable to find column Count" and a pivot into "not
  found", in the TUI as much as over MCP. Column references are now built as
  names rather than parsed as patterns.
- **A row filter that legitimately matched nothing was silently recomputed by a
  different evaluator.** The fallback to per-row evaluation triggered on an empty
  result rather than on the vectorised path failing, so "no rows matched" — an
  answer — was treated as a failure and answered again with different semantics.
- **Retyping a column left aggregators behind that its new type cannot carry.**
  Assigning `sum` to a numeric column and then switching it to String with `t`
  kept the aggregator marked on the column while the footer and every frequency
  table silently skipped it. Incompatible aggregators are now dropped when the
  type changes.

### Added

- **`and`, `or` and `not` in expressions**, so `department == "HR" or department
  == "Marketing"` works wherever an expression does — including the `|` row
  filter, which needed no interface change to gain it. `or` binds loosest, then
  `and`, then `not`. Also `contains(col, regex)`.
- **The MCP server reports any random seed it drew for you**, so an unseeded
  sample or dedup can be repeated by passing the seed back.
- **Window functions.** `zw` adds a column computed from the rows around each
  row — `row_number`, `rank`, `dense_rank`, `cum_sum`, `lag`, `lead`, a group's
  `sum`/`avg`/`min`/`max`/`count` repeated on its rows, and `pct_of_total`. It
  reuses the partition picker `zF` already had, so a window can be scoped to a
  group. In MCP: `{"window": {"fn": "rank", "col": "salary", "over":
  ["department"], "desc": true}}`.

  `cum_sum`, `lag`, `lead` and `row_number` read the rows in their current
  order, so sort first. `zf` and `zF` now call the same function rather than
  building their own window expression.
- **Transposing, deduplicating and random sampling reached MCP**, having lived
  only in the terminal: `{"transpose": {}}`, `{"dedup": {...}}`,
  `{"duplicates": {...}}`, `{"sample": {"n": 100, "seed": 42}}`.
- **`gb` groups by the pinned columns**, computing the aggregates marked with
  `+` on the others. Unlike `F`, which ranks groups by how many rows fall in
  each, this returns exactly the aggregates asked for, in the order asked for.
- **A grand total.** `{"aggregate": [...]}` over MCP answers "what is the total
  revenue" without inventing a grouping column to hang it on.
- **`any_of` gives the MCP filter an OR**, and a predicate may now compare two
  columns: `{"col": "revenue", "op": "gt", "value": {"col": "cost"}}`.
- **Sorting by several keys at once.** `z[` and `z]` add the cursor column as a
  further, less significant key to the sort already running, and the header shows
  each key's rank. In MCP: `{"sort": {"by": [{"col": "region"}, {"col": "amount",
  "desc": true}]}}`.

  Two sorts in sequence are not an equivalent: a single-key sort does not promise
  to preserve the order of rows that tie on its key, so the first ordering may or
  may not survive the second. It happens to survive on small frames, which is
  worse than failing outright.
- The in-app help (`?`) documents sorting, which it never has.

### Changed

- **The two surfaces are now held together by the compiler.** A test classifies
  every one of the 317 terminal actions as sharing a function with a named
  server operation, expressible with what the server has, belonging to the
  interface alone, or a declared gap — and the match is exhaustive, so adding an
  action breaks the build until someone answers "and how does a model do this?".
  A mirror match does the same for every server operation. Two gaps are declared
  today: find & replace in a column, and splitting a column by a delimiter.
- **Random selection takes a seed.** Sampling and the random dedup keeper drew
  from the system source, so the answer could not be repeated — poor form for a
  tool whose numbers get quoted. Both now accept a seed, and MCP reports the one
  it used.
- **Both surfaces now run the same filter.** The terminal's typed expressions and
  the server's structured predicates compile into one expression tree and one
  evaluator, where before they shared nothing — which is how the server ended up
  with a predicate language the terminal could not express, and the terminal kept
  a fallback the server never inherited. Roughly 170 lines of duplicate logic
  went with it. Grouping moved the same way, so `gb` and `group_by` cannot drift.
- Derived tables — frequency, pivot, transpose, group-by, describe — are built
  through one constructor rather than eleven copies of the same struct literal.
  The type correction that one copy had, where an average over an integer column
  is reported as a float rather than inheriting the integer label, now applies to
  all of them.

## [0.6.0] - 2026-08-03

### Added

- **`tuitab --mcp` runs tuitab as an MCP server**, so an AI assistant can compute over a
  data file with the same engine the TUI uses instead of doing the arithmetic itself.
  Register it with a client — for example `claude mcp add tuitab -- tuitab --mcp` — and
  the assistant gets four tools:
  - `tuitab_inspect` — sheets or tables, column names with inferred types, row count,
    sample rows. The first call in any session, so column names are read rather than
    guessed.
  - `tuitab_query` — a pipeline of `filter`, `select`, `sort`, `compute`, `group_by`,
    `frequency`, `pivot`, `join` and `limit`, applied in order. Several pipelines in one
    call share a single load of the file. Results come back as JSON, or go to a file via
    `output.path` in any supported format.
  - `tuitab_describe` — the per-column profile behind the `I` key.
  - `tuitab_jq` — jq programs over nested JSON, JSONL, YAML and TOML.
- There is no SQL and no arbitrary code in the tool surface: the model sends structured
  operations, each one mapping onto a function tuitab already had, and gets back numbers
  Polars computed. The server ships its own documentation — the tool list and usage notes
  reach the model on connect.
- Numbers are returned raw rather than formatted: percentages arrive as fractions, and
  currency as plain numbers. Files written through `output.path` keep the display
  formatting, because those are for a person to read.
- No new dependencies. MCP over stdio is newline-delimited JSON-RPC, which the existing
  `serde_json` already covers, so the TUI build is unchanged in size and in what it pulls
  in.

### Changed

- The per-column profile moved out of the TUI into `data::describe`, shared by the `I`
  key and `tuitab_describe`. Behaviour is unchanged, and the extraction was verified
  against the previous implementation cell by cell.
- **`I` now reports a stable `mode`.** Ties were broken by hash iteration order, so a
  column of unique values gave a different answer on different runs — Rust seeds its
  hasher randomly per process. The most frequent value now wins, and equal counts break
  by first appearance.
- The MCP profile labels the standard deviation `stdev_pop`, because the one `describe`
  computes is the population figure while the footer aggregator of the same name is the
  sample one. The TUI label is unchanged.

## [0.5.0] - 2026-07-28

### Added

- **JSON, JSONL/NDJSON, YAML and TOML open as a table over the real document.**
  The table is a projection of the parsed tree, not a flattened copy: editing a cell
  writes into the document, and saving re-serialises it, so nesting, key order and
  TOML datetimes survive.
- Converting between the structured formats is a different extension on save —
  `config.toml` saved as `config.yaml`. A conversion that cannot carry everything says
  so in the status line.
- **A TOML file saved back as TOML keeps its comments and layout**, because it is
  written through its own source rather than rebuilt.
- `Enter` / `zEnter` dive into the node of the current row or cell; `q`/`Esc` comes
  back. Sheets in a dive chain share one document, so an edit three levels down is
  there when you return, and `U` undoes it at any level.
- `m` cycles how a node is projected: records, key/value, scalars. An object or a TOML
  document opens as key/value rows rather than one very wide row.
- `(` / `)` expand a nested column into `parent.key` / `parent[0]` columns and fold it
  back. Expanding is a view operation — the saved file keeps its real nesting, and
  expanded cells stay editable.
- `E` on a container cell opens its real subtree in `$EDITOR`, which is how keys are
  added, removed and reordered. If the result does not parse, nothing is written and
  the typed text is kept.
- `g/` searches the whole document rather than the rows on screen and lists the hits
  with their paths; `Enter` opens the node with the cursor on the match.
- `gp` jumps to a node by its path, `yp` copies the path of the current cell, and the
  status line shows it when there is nothing more urgent to report.
- `gq` runs a jq query and opens the result as a sheet of its own.
- `zo` on a directory listing reopens a file as an explicitly chosen format, and a file
  with an unhelpful extension is identified by its contents.
- Saving a plain table to a structured format asks which shape to produce — records,
  columns, or key/value — and remembers the answer.
- `--type` now also forces a format for a file, not just for stdin.

### Changed

- JSON no longer goes through the Polars reader, which flattened nesting on save.
- Release binaries are stripped, taking them from 121 MB to 97 MB.
- Operations that would reshape the table without a matching change in the document —
  paste, computed columns, the `z` column operations — are refused on a document sheet
  and point at `E`. Deleting rows, which is a change to the data, removes the array
  element or the key.

### Fixed

- Upgraded calamine, which moves the `.xlsx` parser to a quick-xml without the
  known denial-of-service advisories.

## [0.4.3] - 2026-06-14

### Changed
- Bump dependencies: `rusqlite` 0.31 → 0.40, `calamine` 0.22 → 0.35, `rust_xlsxwriter` 0.80 → 0.95, `rand` 0.9 → 0.10, `serde_json` → 1.0.150. Excel reading migrated to calamine's `Data` cell type and its `Result`-returning `worksheet_range`; random-row selection migrated to rand's renamed `sample` API. No user-facing behaviour change.

### Documentation
- Overhauled the README with animated demos and added a bilingual (English / Русский) documentation set under `docs/`.

## [0.4.2] - 2026-05-05

### Fixed
- Remove `strip = true` from release profile — `cargo install tuitab` no longer requires the `llvm-tools` rustup component

## [0.4.1] - 2026-05-05

### Added
- Status-bar viewport-clip indicator: `[clip 71/80]` when the cursor column's allocated viewport width is smaller than its stored width, so it's clear when content gets cut off purely because the terminal is too narrow

### Changed
- `_` column-width toggle simplified from three modes (Default → Compact → Expanded → Default) to two (Default ↔ Fit). Fit measures content width across all rows; Default restores the load-time bounded width. Header width remains the floor in both
- Column header now has a 1-char left padding so its name doesn't visually touch the previous column's type icon (paired with `column_spacing(0)` to remove the redundant ratatui gap)

### Fixed
- Multi-line cell content (cells containing `\n`, e.g. "Geo allowed list" with newline-separated countries) no longer makes columns expand to full screen width — `calc_column_width` now uses the longest single line instead of summing all lines via `UnicodeWidthStr::width()` on the whole string. Cell rendering also stops at the first `\n` so only the first line is shown
- Drill-down + `q` panic: `Failed to read sheet data from disk: io error` after multiple drill-downs followed by pop. The previous refactor introduced an asymmetric serde impl for `ColumnWidthMode` (auto-derived `Serialize` wrote a `u32` enum index, custom `Deserialize` tried to read a `String` length), causing bincode to read past the swap file's EOF
- `build_column_plan` over-allocated viewport space by 4 chars (highlight symbol `▶ ` and ratatui's default `column_spacing=1` were not subtracted), so the last visible column was silently clipped by ratatui below the width handed to it. Fixed with `max_width = area.width - 4` and explicit `column_spacing(0)` on the table

## [0.4.0] - 2026-04-30

### Added
- `Shift+S` Special select prefix mode with three subcommands:
  - `Shift+S r` — random selection of N visible rows (N entered in popup)
  - `Shift+S d` — select all rows that have an exact duplicate (full row match)
  - `Shift+S D` — smart deduplication: dedup by all columns when no pinned columns; with pinned columns, opens a tiebreaker popup to pick column + ASC/DESC (or random) for choosing which row to keep
- Bulk edit (`ge`) now pre-fills the input with the value of the active cell, so you can quickly tweak or replace it
- Column string operations under `z` prefix:
  - `zr` — find/replace in a column (literal)
  - `zg` — find/replace in a column (regex)
  - `zx` — split a column by delimiter into N new columns
- `ColumnType::FileSize` — integer bytes rendered as human-readable `1.5 KB` / `2.3 MB` / etc.; directory listings now use it so the Size column is sortable numerically
- Three-state column width cycle (`_`): Default (load-time auto-width) → Compact (header-only) → Expanded (full content). Replaces the old binary expand toggle
- Column move mode (`z←` / `z→`): repeated arrows reorder the column until any other key exits

### Changed
- `gt` (toggle all) now performs true per-row inversion: previously selected rows become unselected and vice versa, instead of the old "all-or-nothing" behaviour
- `cargo audit` ignore for `RUSTSEC-2025-0141` (bincode unmaintained warning) — bincode is still pulled transitively by `polars-utils` and we'll drop it once polars upgrades

### Fixed
- Opening a file from a directory listing (`tuitab ~/Downloads/`) when cwd is not the parent directory: previously failed with "No such file or directory" because the relative path was built from the sheet title; now uses the full `source_path` of the directory sheet, and sub-directories propagate `source_path` correctly
- File size in directory listings now displays in human-readable form (B/KB/MB/GB) instead of raw byte count
- `clear_aggregators` and `apply_aggregators` now push undo, so column aggregator changes are reversible

## [0.3.8] - 2026-04-29

### Added
- Chart cursor navigation: `←`/`→` move a highlight across histogram/frequency bars and line-chart points; Enter drills into matching rows
- Histogram drill-down: Enter on a bar opens a filtered table sheet; `q`/Esc returns to the chart
- Pin/unpin (`!`) now restores the column's original position when unpinning
- `bundled-duckdb` Cargo feature (default-enabled): DuckDB is compiled from source by default; pass `--no-default-features` when a system DuckDB library is available to skip the ~5 min C++ compilation

### Fixed
- Save-dialog Tab-completion no longer bleeds expression-autocomplete state — opening Ctrl+S after typing a formula no longer shows formula candidates in the file-path popup
- Chart cursor (`→`) no longer advances past the last bar
- Histogram over a constant-value column now renders and drills down correctly (bin range was too narrow to match any value)
- Chart aggregation selector navigation now wraps at list boundaries, consistent with other selectors

### Changed
- Internal: `handle_action()` decomposed into 8 focused per-domain modules (`chart`, `aggregator`, `edit`, `type_select`, `clipboard`, `io`, `pivot`, `selection`)
- Internal: `table_view::render()` split into `build_column_plan`, `make_header_row`, `make_data_rows`, `make_footer_row`
- Internal: App state extracted into `JoinState`, `ExpressionState`, `ChartState`, `SaveState`, `CopyState`, etc.
- Internal: `ui/popup.rs` and `data/io.rs` split into format-specific sub-modules
- Internal: date constants, comparison helper, and type-conversion helpers moved to the data layer
- Build: `polars` uses `default-features = false`; `arboard` drops unused `image` feature; release profile uses `lto = "thin"`, `strip = true`; SQLite always bundled

## [0.3.7] - 2026-04-24

### Fixed
- Pivot table (`Shift+W`) no longer fails with "explicit column references are not allowed in the `aggregate_function` of `pivot`" when using compound formulas like `sum(col) / count(col)` — replaced `col("pivot_value").first()` with the correct `element().first()` placeholder
- Autocomplete now consistently prioritises prefix matches over substring matches and sorts each group alphabetically
- Column auto-width (`_`) now always expands on first press instead of randomly collapsing — `width_expanded` was incorrectly initialised to `true`, so the first press collapsed the column to header width
- Column auto-width now measures actual displayed values (respecting float precision, currency symbols, percentage formatting) instead of hardcoded per-type estimates
- Groupby (`Shift+F`) and pivot (`Shift+W`) now preserve the source column's display type (`Currency`, `Percentage`, etc.) on the resulting columns — previously all aggregated value columns were downgraded to `Float`

## [0.3.6] - 2026-04-24

### Fixed
- Excel files with duplicate or empty column headers no longer crash on open — empty headers are renamed to `column_N`, duplicate names get a `_2`, `_3`, … suffix

## [0.3.5] - 2026-04-24

### Fixed
- Removed `rust-toolchain.toml` and nightly toolchain experiments that caused `cargo install` failures and forced nightly Rust on all users; CI and release builds are now fully on stable

## [0.3.4] - 2026-04-23

### Fixed
- docs.rs build: remove `components` from `rust-toolchain.toml` so docs.rs correctly applies the pinned `nightly-2026-03-15` toolchain; CI jobs now install `rustfmt`/`clippy` components explicitly

## [0.3.3] - 2026-04-23

### Fixed
- docs.rs build: pin `nightly-2026-03-15` toolchain to work around `polars-ops 0.53` accessing private nightly Rust Unicode APIs (`core::unicode::{Cased, Case_Ignorable}`) removed in nightly ≥ 2026-03-25

## [0.3.2] - 2026-04-23

### Added
- Copy/yank system: `yr` (rows), `yz` (column values), `yZ` (whole column), `yR` (whole table), `yc` (cell) — each opens a format popup (TSV, CSV, JSON, Markdown)
- Redo: `Ctrl+R` complements existing undo (`U` / `Shift+U`)
- JSON export/copy now preserves column order (previously keys were sorted alphabetically)
- Copy and save operations output display-formatted values — percentage columns export as `"30%"`, not `"0.3"`
- `Pct` column in frequency tables (`Shift+F`, `gF`) is now typed as Percentage and displays as `"42.3%"`
- String columns containing `"30%"` values can be converted to Float, Percentage, or Integer via `t` (percent suffix stripped and scaled appropriately)

### Fixed
- Aggregation footer now shows results for Percentage columns on first use (previously showed empty until type was reassigned via `t`)
- Precision resets to 2 decimal places when switching a column to Float/Percentage/Currency type
- Error message when adding an incompatible aggregator now references `t` instead of removed keybindings

## [0.3.1] - 2026-04-22

### Fixed
- Opening a multi-sheet xlsx file directly (e.g. `tuitab file.xlsx`) now shows the sheet overview instead of opening the first sheet

## [0.3.0] - 2026-04-22

### Added
- JOIN contextual sources: pressing `J` now shows sibling items from the same origin — tables from the same SQLite/DuckDB database, files from the same directory, sheets from the same xlsx file
- Multi-select JOIN from overview sheets (directory listing, SQLite/DuckDB/xlsx table browser): select N items with Space, confirm with Enter to chain-join them sequentially
- Chain JOIN: after joining, pressing `J` again continues chaining additional tables onto the result
- xlsx multi-sheet browser: opening an xlsx file with multiple sheets now shows a sheet overview; pressing Enter drills into the selected sheet
- `Shift+E` — open current cell value in `$EDITOR`/`$VISUAL`/`vi`; saves back to the cell if the text was changed
- Tilde expansion (`~/...`) in JOIN path input and save dialog — both file loading and Tab-autocomplete now correctly expand `~` to the home directory
- Tab-completion for JOIN file path input now works with `~/` prefixes

### Fixed
- DuckDB tables with exotic column types (STRUCT, LIST, TIMESTAMP WITH TIME ZONE, etc.) no longer cause a panic on open — all values are read via `CAST(col AS VARCHAR)`

## [0.2.0] - 2026-04-21

### Added
- Save dialog now remembers the original file path and shows relative path by default (e.g., `db/prices.csv` instead of just `prices.csv`)
- Tab-completion for file paths in save dialog — completes to common prefix or cycles through matching files
- DateTime recovery: when converting a Datetime column to Date, the original time is preserved and can be restored when converting back to Datetime
- `date()` function for computed columns — extracts date from Datetime or parses date from string (e.g., `=date(timestamp_col)`)
- File reload via **Shift+R** — reloads current file from disk while preserving scroll position and selection
- Automatic Date/Datetime parsing from string columns supporting multiple input formats (`%Y-%m-%d`, `%Y-%m-%d %H:%M:%S`, ISO 8601, etc.)

### Changed
- Save dialog behavior improved to work correctly with files in subdirectories
- Source path tracking on sheets enables better save/reload functionality

## [0.1.5] - 2026-04-14

### Changed
- polars dependency updated from 0.46 to 0.53; fixes docs.rs build failure caused by `polars-ops 0.46` accessing private nightly Rust stdlib functions (`core::unicode::Case_Ignorable`, `core::unicode::Cased`)

## [0.1.4] - 2026-04-14

### Changed
- docs.rs badge fixed: `documentation` metadata now points to `https://docs.rs/tuitab`; added `[package.metadata.docs.rs]` with `bundled-sqlite` feature so docs.rs builds successfully without a system `libsqlite3`
- Comprehensive rustdoc documentation added: module-level `//!` overviews for `data`, `ui`, `app`, `sheet`, and `theme`; `///` doc comments on public structs, enums, and methods throughout the codebase
- README relative links replaced with absolute GitHub URLs for correct rendering on docs.rs

## [0.1.3] - 2026-04-10

### Added
- Human-readable file sizes in directory listing (e.g. "1.2 KB", "3.4 MB") instead of raw byte counts
- TXT files are now read as a single-column table — each line becomes one row in a "Line" column
- SQLite files now open a table browser showing all tables with name, row count, column count, and SQL definition; pressing Enter drills into the selected table
- Column selection in z-mode: `zs` marks a column with `*`, `zu` unmarks it; pressing `"` with selected columns creates a new sheet containing only those columns (combines with row selection)

### Fixed
- Save dialog (Ctrl+S) now pre-fills with the current sheet's filename instead of the original CLI argument

## [0.1.2] - 2026-04-09

### Added
- Binary alias `ttb` (short for tuitab)

### Fixed
- Missing file error now prints a clean message instead of a backtrace
- `run()` entry point exposed in library crate for external use

## [0.1.1] - 2026-04-09

- Version bump to 0.1.1.

## [0.1.0] - 2026-04-08

### Added

- Multi-format file support: CSV/TSV (auto-delimiter detection), JSON, Parquet, Excel (xlsx/xls), SQLite
- Keyboard-driven navigation: vim-style `hjkl`, `g`/`G` jump, page up/down, column-width cycling
- Row filtering: text search `/`, select by value `,`, expression filter `|!=expr`
- Sorting: ascending/descending on any column, sort reset `r`
- Computed columns via `=expr` with arithmetic, string ops, and date math
- Pivot tables via `W` with column/aggregation autocomplete and input history
- Column statistics via `I`: type, count, nulls, unique, min, max, mean, median, mode, stdev, quantiles (q5–q95)
- Charts via `V`: histogram (Freedman-Diaconis binning), frequency bar chart, line chart (date × numeric), grouped bar chart (category × numeric). Pin a reference column with `!` for two-column charts. Aggregation popup for numeric charts
- Table transpose via `T` (in-place, no phantom columns)
- Frequency table via `F`
- Row selection, yank/paste, delete
- Sheet-from-selection via `"`
- Clipboard integration
- Column aggregators in footer (sum, count, avg, median, stdev, percentiles)
- Column type assignment via `t`
- Export to CSV/Parquet/Excel via Ctrl+S
- Pipe mode: `cat data.csv | tuitab -t csv`
- Everforest dark colour theme
- Non-English keyboard remapping
- Three binary aliases: `tuitab`, `ttab`, `tt`

[Unreleased]: https://github.com/denisotree/tuitab/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/denisotree/tuitab/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/denisotree/tuitab/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/denisotree/tuitab/compare/v0.4.3...v0.5.0
[0.4.3]: https://github.com/denisotree/tuitab/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/denisotree/tuitab/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/denisotree/tuitab/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/denisotree/tuitab/compare/v0.3.8...v0.4.0
[0.3.8]: https://github.com/denisotree/tuitab/compare/v0.3.7...v0.3.8
[0.3.7]: https://github.com/denisotree/tuitab/compare/v0.3.6...v0.3.7
[0.3.6]: https://github.com/denisotree/tuitab/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/denisotree/tuitab/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/denisotree/tuitab/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/denisotree/tuitab/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/denisotree/tuitab/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/denisotree/tuitab/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/denisotree/tuitab/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/denisotree/tuitab/compare/v0.1.5...v0.2.0
[0.1.5]: https://github.com/denisotree/tuitab/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/denisotree/tuitab/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/denisotree/tuitab/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/denisotree/tuitab/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/denisotree/tuitab/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/denisotree/tuitab/releases/tag/v0.1.0
