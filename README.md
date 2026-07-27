# tuitab

<div align="center">

[![CI](https://github.com/denisotree/tuitab/actions/workflows/ci.yml/badge.svg)](https://github.com/denisotree/tuitab/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/tuitab.svg)](https://crates.io/crates/tuitab)
[![docs.rs](https://img.shields.io/docsrs/tuitab)](https://docs.rs/tuitab)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/denisotree/tuitab/blob/master/LICENSE)

**A fast, keyboard-driven terminal explorer for tabular data.**

Open **CSV · JSON · YAML · TOML · Parquet · Excel · SQLite · DuckDB** straight from your shell —
filter, sort, pivot, join, compute columns, and chart distributions without
leaving the terminal.

![tuitab demo — open, sort, chart, describe](https://raw.githubusercontent.com/denisotree/tuitab/master/.github/assets/demo.gif)

</div>

```sh
tuitab data.csv                   # open a file
tuitab config.toml                # JSON / YAML / TOML open as a browsable tree
tuitab -t yaml deploy.conf        # force a format when the extension lies
tuitab orders.csv customers.csv   # browse several files as a list
cat data.csv | tuitab -t csv      # read from a pipe
```

> **New to tuitab?** Jump to the [Quick start](#quick-start), or read the full
> [**Documentation**](#documentation) — available in **English** and **Русский**.

---

## Highlights

- **Tabular and structured formats** — CSV/TSV (auto-delimiter), JSON, JSONL/NDJSON,
  YAML, TOML, Parquet, Excel (xlsx/xls), SQLite, DuckDB. Browse a whole directory, or
  pipe data in over stdin.
- **Nested data, edited in place** — JSON/YAML/TOML open as a table over the real
  document: `Enter` dives into a nested object or list, `m` switches between
  record and key/value layouts, and edits write back into the document, not into a
  flattened copy. Saving re-serialises the tree, so structure survives.
- **Vim-style navigation** — `hjkl`, `gg`/`G`, page jumps, sticky pinned columns.
- **Instant analysis** — per-column statistics, frequency tables, and charts
  (histogram, bar, line, grouped bar) rendered right in the terminal.
- **Reshape on the fly** — pivot tables, JOINs across files, transpose, computed
  columns from an expression language.
- **Clean, fast, type-aware** — Polars-backed engine, Everforest theme, undo/redo,
  currency / percentage / date column types.
- **Export and convert** — write back to CSV, TSV, Parquet, JSON, JSONL, YAML, TOML,
  Excel, or SQLite. Converting between structured formats is just a different
  extension: open `config.toml`, save as `config.yaml`. Yank rows to the clipboard as
  TSV, CSV, JSON, or Markdown.

---

## See it in action

### Charts — histogram, bar, line, grouped bar

Press `V` on any column. Numeric columns get a Freedman–Diaconis histogram;
categorical columns get a frequency bar chart. [Pin](#keybindings) a date or
category column first with `!` to draw line charts and grouped bars.

![Charts](https://raw.githubusercontent.com/denisotree/tuitab/master/.github/assets/charts.gif)

### Pivot tables

Pin the column(s) to group by, place the cursor on the column to spread across,
press `W`, and type an aggregation formula such as `sum(revenue)`.

![Pivot](https://raw.githubusercontent.com/denisotree/tuitab/master/.github/assets/pivot.gif)

### JOIN across files

Press `J` for a step-by-step wizard: pick another file (or an open sheet),
choose `INNER` / `LEFT` / `RIGHT` / `OUTER`, and select the key columns.

![JOIN](https://raw.githubusercontent.com/denisotree/tuitab/master/.github/assets/join.gif)

### Computed columns

Press `=` and type an expression. Arithmetic, string and date functions, and
conditionals are all supported — the new column appears right next to the cursor.

![Computed columns](https://raw.githubusercontent.com/denisotree/tuitab/master/.github/assets/compute.gif)

---

## Installation

### Cargo (crates.io)

```sh
cargo install tuitab
```

Installs three commands: `tuitab`, plus the shorter aliases `ttab` and `ttb`.

### Homebrew (macOS / Linux)

```sh
brew tap denisotree/tuitab
brew install tuitab
```

### Arch Linux (AUR)

```sh
yay -S tuitab          # pre-built binary: tuitab-bin
# or build from source:
git clone https://aur.archlinux.org/tuitab.git && cd tuitab && makepkg -si
```

### Debian / Ubuntu

Download the `.deb` from the [Releases page](https://github.com/denisotree/tuitab/releases), then:

```sh
sudo dpkg -i tuitab_*_amd64.deb
```

### Pre-built binaries

Grab a tarball for your platform (Linux / macOS, x86_64 / aarch64) from the
[Releases page](https://github.com/denisotree/tuitab/releases).

<details>
<summary><b>Building from source</b></summary>

```sh
cargo build --release
```

The default build bundles DuckDB and SQLite from source, so the binary is fully
self-contained — no system libraries required. This compiles DuckDB's C++ core
(~5 min the first time).

To skip that and link a **system** DuckDB instead:

```sh
brew install duckdb                # macOS
sudo apt install libduckdb-dev     # Debian / Ubuntu
cargo build --release --no-default-features
```

| Feature | Default | Description |
|---------|:-------:|-------------|
| `bundled-duckdb` | ✓ | Compile DuckDB from source; no system `libduckdb` needed |

</details>

---

## Quick start

```sh
cargo install tuitab
tuitab data.csv
```

- Move with `h` `j` `k` `l`; jump with `gg` / `G`.
- Sort the current column: `[` ascending, `]` descending, `r` to reset.
- Chart it: `V`. Per-column stats: `I`. Frequency table: `F`.
- Select rows by an expression: `|!=amount > 1000`.
- Add a column: `=` then e.g. `revenue / units`.
- Save / export: `Ctrl+S`. Quit: `q`. Help at any time: `?`.

---

## Usage

```text
tuitab [OPTIONS] [FILES]...

Arguments:
  [FILES]...  One or more files to open, a directory, or '-' for stdin.
              Pass multiple files to browse them as a list.
              Defaults to the current directory.

Options:
  -d, --delimiter <CHAR>   Column delimiter (auto-detected if omitted)
  -t, --type <FORMAT>      Format when reading from stdin: csv, tsv, txt, json
  -h, --help               Print help
  -V, --version            Print version
```

### Browse several files

```sh
tuitab orders.csv customers.csv products.parquet
```

A directory-style listing opens with each file as a row. Press `Enter` to open
one; `Esc` or `q` to go back.

### Pipe mode

```sh
psql -c "SELECT * FROM orders" --csv | tuitab -t csv
sqlite3 app.db ".mode csv" ".headers on" "SELECT * FROM users" | tuitab -t csv
```

> Stdin accepts `csv`, `tsv`, `txt`, `json`, `jsonl`, `yaml`, and `toml`. For
> Parquet/Excel/SQLite, open the file directly.

---

## Keybindings

The essentials — see the [full keybinding reference](https://github.com/denisotree/tuitab/blob/master/docs/en/keybindings.md)
for every command (column ops, clipboard, dedup, and more).

| Key | Action | Key | Action |
|-----|--------|-----|--------|
| `h` `j` `k` `l` | Move cursor | `[` / `]` | Sort asc / desc |
| `gg` / `G` | First / last row | `r` | Reset sort |
| `Ctrl+B` / `Ctrl+F` | Page up / down | `/` | Search (regex) |
| `!` | Pin / unpin column | `\|` | Select rows by regex / expression |
| `=` | Add computed column | `,` | Select rows by value |
| `V` | Chart column | `s` / `u` | Select / unselect row |
| `I` | Column statistics | `+` / `-` | Add / clear aggregator |
| `F` | Frequency table | `t` | Set column type |
| `W` | Pivot table | `Enter` | Transpose row / drill down / dive into node |
| `J` | JOIN with another table | `T` | Transpose table |
| `m` | Cycle JSON/YAML/TOML layout | `zEnter` | Dive into the node in this cell |
| `E` | Edit cell (or node) in `$EDITOR` | `Ctrl+S` | Save / export / convert |
| `U` / `Ctrl+R` | Undo / redo | | |
| `?` | Help | `q` | Quit / pop sheet |

> Non-QWERTY layouts (ЙЦУКЕН, QWERTZ, AZERTY) are transparently remapped, so the
> hotkeys work regardless of your keyboard.

---

## Documentation

Full guides, organised by topic, in two languages:

| 🇬🇧 English | 🇷🇺 Русский |
|------------|------------|
| [Documentation index](https://github.com/denisotree/tuitab/blob/master/docs/en/README.md) | [Оглавление документации](https://github.com/denisotree/tuitab/blob/master/docs/ru/README.md) |
| [Getting started](https://github.com/denisotree/tuitab/blob/master/docs/en/getting-started.md) | [Начало работы](https://github.com/denisotree/tuitab/blob/master/docs/ru/getting-started.md) |
| [Keybindings](https://github.com/denisotree/tuitab/blob/master/docs/en/keybindings.md) | [Горячие клавиши](https://github.com/denisotree/tuitab/blob/master/docs/ru/keybindings.md) |
| [Expressions](https://github.com/denisotree/tuitab/blob/master/docs/en/expressions.md) | [Выражения](https://github.com/denisotree/tuitab/blob/master/docs/ru/expressions.md) |
| [Charts](https://github.com/denisotree/tuitab/blob/master/docs/en/charts.md) | [Графики](https://github.com/denisotree/tuitab/blob/master/docs/ru/charts.md) |
| [JOIN](https://github.com/denisotree/tuitab/blob/master/docs/en/join.md) | [JOIN](https://github.com/denisotree/tuitab/blob/master/docs/ru/join.md) |
| [Pivot tables](https://github.com/denisotree/tuitab/blob/master/docs/en/pivot.md) | [Сводные таблицы](https://github.com/denisotree/tuitab/blob/master/docs/ru/pivot.md) |
| [Recipes](https://github.com/denisotree/tuitab/blob/master/docs/en/recipes.md) | [Рецепты](https://github.com/denisotree/tuitab/blob/master/docs/ru/recipes.md) |

---

## Acknowledgements

tuitab is inspired by [VisiData](https://www.visidata.org) — a brilliant terminal
spreadsheet multitool by [Saul Pwanson](https://github.com/saulpw). If you find
tuitab useful, check out VisiData too. Built with
[ratatui](https://github.com/ratatui/ratatui), [Polars](https://www.pola.rs),
and [crossterm](https://github.com/crossterm-rs/crossterm).

## Contributing

Bug reports, feature requests, and pull requests are welcome.
See [CONTRIBUTING.md](https://github.com/denisotree/tuitab/blob/master/CONTRIBUTING.md).

## License

Apache-2.0 — see [LICENSE](https://github.com/denisotree/tuitab/blob/master/LICENSE).
