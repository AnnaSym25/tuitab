# JOIN

> 🇷🇺 [Эта страница на русском](../ru/join.md) · [← Documentation index](README.md)

Press `J` to combine the current table with another one through a step-by-step
wizard. The other table can be **any file tuitab opens** (CSV, TSV, JSON, JSONL,
YAML, TOML, Parquet, Arrow, Excel, Markdown, SQLite, DuckDB) or another sheet you
already have open.

![JOIN wizard](https://raw.githubusercontent.com/denisotree/tuitab/master/.github/assets/join.gif)

## The four steps

### 1. Pick the right-hand table

A popup lists:

- **`[Browse file…]`** — type a path (`Tab` autocompletes) to any supported file.
- **Open sheets** — any other sheets already on your stack.

### 2. Choose the join type

| Option | SQL equivalent | Rows kept |
|--------|----------------|-----------|
| `INNER` | `INNER JOIN` | Only rows with a match in both tables |
| `LEFT` | `LEFT JOIN` | All left rows; unmatched right cells are null |
| `RIGHT` | `RIGHT JOIN` | All right rows; unmatched left cells are null |
| `OUTER` | `FULL OUTER JOIN` | All rows from both tables |

### 3. Select the left key columns

A checkbox list of the current table's columns. Toggle with `Space`; the
**order** you pick them in matters — left key 1 matches right key 1, and so on.
Press `Enter` to continue.

### 4. Select the right key columns

The same list for the other table. Columns whose names match your left keys are
pre-selected. Adjust and press `Enter` to run the join.

> The key counts must match: two left keys ⇒ exactly two right keys. A mismatch
> shows an error in the status bar.

## Result

A new sheet is pushed onto the stack titled `left JOIN right`. Press `Esc` / `q`
to pop back to the original table. Non-key columns that exist in both tables get
a `_right` suffix so nothing is overwritten.

## Worked example

```sh
# orders.csv:    order_id, customer_id, amount
# customers.csv: customer_id, name, country
tuitab orders.csv
```

1. Press `J`.
2. Choose **`[Browse file…]`**, type `customers.csv`, press `Enter`.
3. Choose **LEFT**, press `Enter`.
4. Toggle `customer_id` on the left, press `Enter`.
5. Toggle `customer_id` on the right, press `Enter`.

The result is every order enriched with its customer's `name` and `country`.

## See also

- [Pivot tables](pivot.md) to summarise the joined result.
- [Keybindings](keybindings.md) for sheet navigation (`Esc` / `q` to pop back).
