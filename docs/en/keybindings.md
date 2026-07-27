# Keybindings

> 🇷🇺 [Эта страница на русском](../ru/keybindings.md) · [← Documentation index](README.md)

The complete command reference. Press `?` in tuitab for a built-in summary.
All keys below are for **Normal mode** unless noted. Many commands act on the
**cursor column** or the **selected rows**.

## Navigation

| Key | Action |
|-----|--------|
| `h` `j` `k` `l` / arrows | Move cursor left / down / up / right |
| `gg` | Jump to the first row |
| `G` / `End` | Jump to the last row |
| `Home` | Jump to the first row |
| `Ctrl+F` / `PageDown` | Page down |
| `Ctrl+B` / `PageUp` | Page up |
| `q` / `Esc` | Quit — or pop back one sheet if you've drilled in |

## Sorting

| Key | Action |
|-----|--------|
| `[` | Sort ascending by the cursor column |
| `]` | Sort descending by the cursor column |
| `r` | Reset to the original row order |

## Search & row selection

| Key | Action |
|-----|--------|
| `/` | Search — highlights cells matching a regex |
| `n` / `N` | Jump to next / previous match |
| `c` | Clear search highlights |
| `,` | Select rows whose cell equals the current value |
| `\|` | Select rows by regex, or by expression with the `!=` prefix (e.g. `\|!=age > 30`) |
| `s` / `u` | Select / unselect the current row |
| `gs` / `gu` | Select all / unselect all rows |
| `gt` | Invert the current selection |
| `Shift+S` `r` | Select **N** random rows |
| `Shift+S` `d` | Select all rows that have duplicates |
| `Shift+S` `D` | Smart dedup — keeps one row per group (asks for a tiebreaker if columns are pinned) |
| `d` | Delete the selected rows |
| `"` | Create a new sheet from the selected rows |

See [Expressions](expressions.md) for the `|!=` filter language.

## Columns

| Key | Action |
|-----|--------|
| `!` (`Shift+1`) | Pin / unpin the column (stays visible when scrolling; used by charts & pivot) |
| `_` | Cycle the column width |
| `g_` | Cycle widths for all columns |
| `=` | Add a computed column from an [expression](expressions.md) |
| `t` | Open the column **type** menu (String, Integer, Float, Date, Datetime, Boolean, Percentage, Currency, File size) — choosing **Currency** lets you pick USD / EUR / GBP / JPY |
| `+` | Add a column footer aggregator (multi-select with `Space`) |
| `-` | Clear the column's aggregators |
| `Z` | Quick-aggregate the visible / selected values into the status bar |

### Column operations (`z` prefix)

| Key | Action |
|-----|--------|
| `ze` | Rename the column |
| `zd` | Delete the column |
| `zi` | Insert an empty column |
| `zs` / `zu` | Mark / unmark the column (marked columns are shown with `*` and respected by copy) |
| `z←` / `zh` | Move the column left |
| `z→` / `zl` | Move the column right |
| `z.` / `z>` | Increase decimal precision |
| `z,` / `z<` | Decrease decimal precision |
| `zf` | Add a "% of total" column |
| `zF` | Add partitioned "% of total" columns (pick the partition columns) |
| `zr` | Find & replace text in the column |
| `zg` | Find & replace with a regex |
| `zx` | Split the column by a delimiter |
| `zEnter` | Dive into the node in the current cell (JSON/YAML/TOML) |

## Rows, sheets & analytics

| Key | Action |
|-----|--------|
| `Enter` | Transpose the current row into a Column/Value view (drill-down); on a JSON/YAML/TOML sheet, dive into the row's node |
| `T` | Transpose the whole table (press again to undo) |
| `I` | Describe sheet — per-column statistics |
| `F` | Frequency table for the cursor column |
| `gF` | Multi-column frequency table (groups by the pinned columns) |
| `gD` | Deduplicate by the pinned columns (keeps the first row per group) |
| `V` | [Chart](charts.md) the cursor column |
| `W` | [Pivot table](pivot.md) |
| `J` | [JOIN](join.md) with another table |
| `e` | Edit the current cell |
| `E` | Edit the current cell in `$EDITOR`; on a JSON/YAML/TOML sheet a container cell opens as its real subtree, so keys can be added, removed and reordered |
| `m` | Cycle how the current node is projected: records / key-value / scalars |
| `(` / `)` | Expand a nested column into one column per key, or fold it back |
| `ge` | Bulk-edit — set the same value on every selected row |

## Clipboard (`y` prefix)

| Key | Action |
|-----|--------|
| `yc` | Copy the current cell |
| `yr` | Copy selected rows (or the current row) — pick a format |
| `yz` | Copy the cursor column for the selected rows — pick a format |
| `yZ` | Copy the entire cursor column — pick a format |
| `yR` | Copy the entire table — pick a format |
| `p` | Paste rows from the clipboard |

Row/table copies offer **TSV · CSV · JSON · Markdown**; column copies offer
newline-, comma-, or quoted-comma-separated. `yr` and `yR` respect columns
marked with `zs`.

## File

| Key | Action |
|-----|--------|
| `Ctrl+S` | Save / export / convert (CSV, TSV, Parquet, JSON, JSONL, YAML, TOML, Excel, or SQLite) |
| `R` | Reload the file from disk |
| `U` / `Shift+U` | Undo (up to 50 steps) |
| `Ctrl+R` | Redo |
| `?` | Toggle the help overlay |

## Charts view

| Key | Action |
|-----|--------|
| `h` `k` / `←` `↑` | Previous bar / series |
| `l` `j` / `→` `↓` | Next bar / series |
| `Enter` | Drill down into the selected bar |
| `V` / `q` / `Esc` | Close the chart |

## Input modes

When you're typing into a prompt (search, expression, pivot formula, rename,
save path, …):

| Key | Action |
|-----|--------|
| `←` `→` `Home` `End` | Move the text cursor |
| `Backspace` / `Delete` | Delete left / right |
| `Tab` | Autocomplete (column names / file paths, where available) |
| `↑` / `↓` | Previous / next entry in history (expression & pivot inputs) |
| `Enter` | Apply |
| `Esc` | Cancel |


## JSON / YAML / TOML sheets

These formats open as a table over the real document, not as a flattened copy.

| Key | Action |
|-----|--------|
| `Enter` | Dive into the node of the current row |
| `zEnter` | Dive into the node of the current cell |
| `q` / `Esc` | Go back up one level |
| `m` | Cycle the layout: records / key-value / scalars |
| `(` | Expand the cursor column of containers into `parent.key` / `parent[0]` columns |
| `)` | Fold the innermost expansion back into one column |
| `e` | Edit a scalar — the value keeps its type, and a string stays a string |
| `E` | Edit a container as text in `$EDITOR` (add, remove or reorder keys) |
| `Ctrl+S` | Save; a different extension converts the format |

Every sheet in a dive chain shares one document, so an edit made three levels down is
there when you come back up, and `U` undoes it at any level.

Expanding is a view operation: `(` never changes the document, so saving still writes
the real nesting rather than flattened `parent.key` names. Expanded cells stay editable
and write to the node they actually address.

Saving re-serialises the document, so nesting, key order and TOML datetimes survive.
Comments and original formatting do not — they are not part of the parsed document.

## Keyboard layouts

Non-QWERTY layouts — **ЙЦУКЕН** (Russian), **QWERTZ** (German), and **AZERTY**
(French) — are transparently remapped to their QWERTY positions, so the hotkeys
work without switching layout.
