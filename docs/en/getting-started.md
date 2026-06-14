# Getting started

> 🇷🇺 [Эта страница на русском](../ru/getting-started.md) · [← Documentation index](README.md)

## Install

The quickest route is Cargo:

```sh
cargo install tuitab
```

This installs three identical commands — `tuitab` and the shorter aliases `ttab`
and `ttb`. Other options (Homebrew, AUR, Debian, pre-built binaries) are in the
[main README](https://github.com/denisotree/tuitab/blob/master/README.md#installation).

## Open a file

Pass any supported file as the first argument:

```sh
tuitab data.csv
tuitab report.parquet
tuitab books.xlsx
tuitab app.db
```

| Format | Extensions | Notes |
|--------|-----------|-------|
| CSV / TSV | `.csv`, `.tsv` | Delimiter auto-detected; override with `-d` |
| JSON | `.json` | Array of objects, or newline-delimited |
| Parquet | `.parquet` | Columnar, fast |
| Excel | `.xlsx`, `.xls` | Multi-sheet workbooks open as a sheet list |
| SQLite | `.sqlite`, `.sqlite3`, `.db` | Tables open as a table list |
| DuckDB | `.duckdb`, `.ddb`, `.db` | `.db` is probed as SQLite first, then DuckDB |

For workbooks and databases, tuitab first shows an overview (sheets or tables);
press `Enter` on a row to open it, `Esc` / `q` to go back.

### Override the delimiter

When auto-detection guesses wrong (or you have a `;`-separated file):

```sh
tuitab data.csv -d ';'
```

## Browse several files

Pass more than one path and tuitab opens a **file list** instead of a single
table:

```sh
tuitab orders.csv customers.csv products.parquet
```

Each file becomes a row showing its name, size, and modified time. `Enter` opens
the highlighted file; `Esc` / `q` returns to the list.

## Browse a directory

Point tuitab at a folder (or run it with no arguments to use the current one):

```sh
tuitab ./reports/
tuitab               # current directory
```

You get a navigable file browser — open files with `Enter`, step back out with
`Esc` / `q`.

## Read from a pipe

tuitab reads from stdin when data is piped in. Tell it the format with `-t`:

```sh
cat data.csv | tuitab -t csv
psql -c "SELECT * FROM orders" --csv | tuitab -t csv
echo '[{"id":1,"name":"Alice"}]'   | tuitab -t json
```

Stdin understands `csv`, `tsv`, `txt`, and `json`. (`txt` is treated as CSV.)
To read Parquet, Excel, SQLite, or DuckDB, open the file by path instead of
piping it.

## The screen

```text
┌ sample.csv ───────────────────────────────────────┐
│  id  s│ name        s│ age #│ salary  ~│ department s│   ← header row (name + type icon)
│▌ 1    │ Alice Johnson│ 30   │ 75000.00 │ Engineering │   ← cursor row (highlighted)
│  2    │ Bob Smith    │ 45   │ 92000.50 │ Management  │
│  …                                                  │
├─────────────────────────────────────────────────────┤
│ NORMAL   Loaded 20 rows                  row 3/20 col 3/5 │  ← status bar
└─────────────────────────────────────────────────────┘
```

- The **header** shows each column name plus a small **type icon**
  (`s` string, `#` integer, `~` float, `d` date, `t` datetime, `%` percentage,
  `$` currency, `B` file size, `?` boolean).
- The **status bar** shows the current mode, a message, and your position.
- Press `?` at any time for the in-app help overlay.

## Next steps

- Learn the [keybindings](keybindings.md).
- Try [charts](charts.md), [pivot tables](pivot.md), and [JOINs](join.md).
- Browse task-oriented [recipes](recipes.md).
