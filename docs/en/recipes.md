# Recipes

> 🇷🇺 [Эта страница на русском](../ru/recipes.md) · [← Documentation index](README.md)

Short, task-oriented how-tos. See [Keybindings](keybindings.md) for the full key
list.

## Summarise a column

- **Statistics** — press `I` for a Describe sheet: count, nulls, unique, min,
  max, mean, median, mode, stdev, and quantiles per column.
- **Frequency** — press `F` for value counts of the cursor column. Pin several
  columns and press `gF` for a multi-column frequency.
- **Footer aggregator** — press `+`, multi-select with `Space` (sum, avg,
  median, count, distinct, percentiles, min, max, …). Clear with `-`.
- **Quick total** — press `Z` to aggregate the visible / selected values into
  the status bar.

## Find & remove duplicates

- **Highlight duplicates** — `Shift+S` then `d` selects every row that has a
  duplicate.
- **Keep one per group** — `Shift+S` then `D` does a smart dedup; if you've
  pinned columns, it asks for a tiebreaker to decide which row to keep.
- **Drop by key** — pin the key columns, then `gD` keeps the first row per group.
- After selecting, press `d` to delete or `"` to split the selection into a new
  sheet.

## Compute a share of total

- **% of total** — on a numeric column, press `zf` to add a "% of total" column.
- **% within groups** — press `zF`, choose the partition columns, and get a
  share-of-partition column.

## Reshape

- **Transpose a row** — press `Enter` to flip the current row into a
  Column / Value view (good for wide records).
- **Transpose the table** — press `T` to swap rows and columns; press `T` again
  to undo.
- **Pivot** — see [Pivot tables](pivot.md).
- **Random sample** — `Shift+S` then `r`, enter how many rows.

## Edit data

- **One cell** — press `e` (or `E` to open it in `$EDITOR`).
- **Many rows at once** — select rows, then `ge` to set the same value on all of
  them.
- **Rename a column** — `ze`. **Delete** — `zd`. **Insert blank** — `zi`.
- **Find & replace** — `zr` (literal) or `zg` (regex) within the column.
- **Split a column** — `zx`, then enter the delimiter.
- **Decimal places** — `z.` more, `z,` fewer.

## Set column types

Press `t` to pick a type: String, Integer, Float, Date, Datetime, Boolean,
Percentage, Currency, or File size. Choosing **Currency** then lets you pick
**USD · EUR · GBP · JPY**, which formats the column with the right symbol.

## Copy to the clipboard

| Keys | Copies |
|------|--------|
| `yc` | The current cell |
| `yr` | Selected rows (or current row) — choose TSV / CSV / JSON / Markdown |
| `yz` | The cursor column for selected rows |
| `yZ` | The whole cursor column |
| `yR` | The whole table — choose TSV / CSV / JSON / Markdown |

Mark columns with `zs` to include only those in `yr` / `yR`. Paste rows back with
`p`.

## Export to a file

Press `Ctrl+S` and enter a path. The format follows the extension:

| Extension | Format |
|-----------|--------|
| `.csv` / `.tsv` | CSV / TSV |
| `.parquet` | Parquet |
| `.json` | JSON |
| `.xlsx` | Excel |
| `.db` / `.sqlite` / `.sqlite3` | SQLite |

`Tab` autocompletes the path. Undo any edit with `U`, redo with `Ctrl+R`, or
reload from disk with `R`.

## Filter to what matters

- **Search** — `/` highlights cells matching a regex; `n` / `N` to step through.
- **Select by value** — `,` selects rows equal to the current cell.
- **Select by expression** — `|` then `!=expr`, e.g. `|!=amount > 1000`
  (see [Expressions](expressions.md)).
- Then `"` to make a sheet from the selection, or `d` to delete it.
