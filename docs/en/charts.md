# Charts

> 🇷🇺 [Эта страница на русском](../ru/charts.md) · [← Documentation index](README.md)

Press `V` on any column to draw a chart. What you get depends on the column's
type and whether you've **pinned** a reference column with `!`.

![Charts — histogram and grouped bar](https://raw.githubusercontent.com/denisotree/tuitab/master/.github/assets/charts.gif)

## Chart types

| Cursor column | Pinned column (`!`) | Chart |
|---------------|---------------------|-------|
| Numeric | — | **Histogram** (Freedman–Diaconis bins) |
| Categorical | — | **Frequency bar chart** |
| Numeric | Date / Datetime | **Line chart** (choose an aggregation) |
| Categorical | Date / Datetime | **Line chart** (count over time) |
| Numeric | Categorical | **Grouped bar chart** (choose an aggregation) |

When a chart aggregates, a small popup asks how:
**Sum · Count · Avg · Median · Min · Max**. Pick with `j` / `k` and `Enter`.

## How pinning shapes the chart

Pinning (`!`) sets the **reference axis**:

- Pin a **date** column, move the cursor to a **numeric** column, press `V` →
  a line chart of that value over time.
- Pin a **category** column (e.g. region), move the cursor to a **numeric**
  column, press `V` → a grouped bar chart (one bar per category).
- Pin nothing and press `V` → a histogram (numeric) or frequency chart
  (categorical) of the cursor column alone.

Unpin with `!` again. See [keybindings](keybindings.md) for pinning.

## Inside the chart

| Key | Action |
|-----|--------|
| `h` `k` / `←` `↑` | Select the previous bar / point |
| `l` `j` / `→` `↓` | Select the next bar / point |
| `Enter` | **Drill down** — filter the table to the selected bar's rows |
| `V` / `q` / `Esc` | Close the chart |

Each bar is labelled with its value, and the chart automatically switches
between vertical and horizontal layout depending on how long the category
labels are.

## Examples

```sh
tuitab sales.csv     # columns: date, region, category, units, revenue
```

- **Distribution of revenue** — move to `revenue`, press `V`. A histogram with
  Freedman–Diaconis bins appears.
- **Revenue by region** — pin `region` with `!`, move to `revenue`, press `V`,
  choose **Sum**. A grouped bar chart appears.
- **Revenue over time** — pin `date` with `!`, move to `revenue`, press `V`,
  choose **Sum**. A line chart appears.
- **Category mix** — move to `category`, press `V` for a frequency bar chart.

## See also

- [Pivot tables](pivot.md) — cross-tabulate the same data as a grid.
- [Recipes](recipes.md) — statistics (`I`) and frequency tables (`F`).
