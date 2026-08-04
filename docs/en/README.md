# tuitab documentation

A fast, keyboard-driven terminal explorer for tabular data —
**CSV · JSON · Parquet · Excel · SQLite · DuckDB**.

> 🇷🇺 Эта документация также доступна на [русском языке](../ru/README.md).

## Contents

| Guide | What's inside |
|-------|---------------|
| [Getting started](getting-started.md) | Install, open files, directory & multi-file browsing, stdin/pipe mode |
| [Keybindings](keybindings.md) | The complete command reference, grouped by mode |
| [Expressions](expressions.md) | Computed columns and expression filters: operators, functions, dates |
| [Charts](charts.md) | Histogram, frequency, line, and grouped-bar charts; pinning & drill-down |
| [JOIN](join.md) | The step-by-step JOIN wizard and join types |
| [Pivot tables](pivot.md) | Pivot syntax, aggregations, and worked examples |
| [MCP server](mcp.md) | Letting an AI assistant compute with tuitab's engine: tools, operations, connecting |
| [Recipes](recipes.md) | Task-oriented how-tos: dedup, % of total, clipboard, export, and more |

## At a glance

```sh
tuitab data.csv                   # open a file
tuitab orders.csv customers.csv   # browse several files as a list
tuitab ./reports/                 # browse a directory
cat data.csv | tuitab -t csv      # read from a pipe
```

Once a table is open: move with `hjkl`, sort with `[` / `]`, chart with `V`,
get statistics with `I`, and press `?` for in-app help. Full details in
[Keybindings](keybindings.md).

## See also

- [Main project README](https://github.com/denisotree/tuitab/blob/master/README.md)
- [Contributing guide](https://github.com/denisotree/tuitab/blob/master/CONTRIBUTING.md)
- [Changelog](https://github.com/denisotree/tuitab/blob/master/CHANGELOG.md)
