# Expressions

> 🇷🇺 [Эта страница на русском](../ru/expressions.md) · [← Documentation index](README.md)

tuitab has a small expression language used in two places:

- **Computed columns** — press `=` and type an expression to add a new column.
- **Expression row-select** — press `|`, then `!=` followed by an expression, to
  select every row where the expression is true (e.g. `|!=amount > 1000`).

![Adding a computed column](https://raw.githubusercontent.com/denisotree/tuitab/master/.github/assets/compute.gif)

## Referencing columns

Write a column name directly. Bare names work when they have no spaces or
special characters:

```text
revenue / units
age * 2
```

In the input prompt, press `Tab` to autocomplete column names.

## Literals

| Kind | Examples |
|------|----------|
| Number | `42`, `3.14`, `-5` |
| String | `"Engineering"`, `"N/A"` |
| Date | `2024-01-31` (`YYYY-MM-DD`) |

## Operators

| Category | Operators |
|----------|-----------|
| Arithmetic | `+` `-` `*` `/` |
| Comparison | `==` `!=` `<` `>` `<=` `>=` |
| Membership | `value in (a, b, c)` |

`+` also concatenates when applied to strings. Comparisons produce a boolean,
which is what the `|!=` row-select uses.

```text
salary > 90000
department == "Engineering"
status != "closed"
region in ("North", "South")
contains(name, "^A")
```

Combine conditions with `and`, `or` and `not`. `or` binds loosest, then `and`,
then `not`, so `a and b or c` reads as `(a and b) or c` — parenthesise when you
mean otherwise:

```text
department == "Engineering" and age > 40
department == "HR" or department == "Marketing"
not (status == "closed")
(region == "North" or region == "South") and amount > 1000
```

> An operand that is not a genuine boolean makes the whole clause null rather
> than quietly counting as false, so a broken comparison cannot be negated into
> a confident answer.

## Functions

### Strings

| Function | Result |
|----------|--------|
| `concat(a, b, …)` | Join values into one string |
| `split(s, delim)` | First field of `s` split on `delim` |
| `substring(s, start, len)` | Substring (0-based start) |
| `len(s)` | Length of the string |

### Dates & time

| Function | Result |
|----------|--------|
| `year(d)` `month(d)` `day(d)` | Parts of a date |
| `hour(dt)` `minute(dt)` | Parts of a datetime |
| `today()` | Current date |
| `now()` | Current datetime |
| `date(s)` | Parse a value into a date |
| `date_format(d, fmt)` | Format a date with a `chrono`/`strftime` pattern |

### Conditionals

| Function | Result |
|----------|--------|
| `if(cond, a, b)` | `a` when `cond` is true, else `b` |

```text
if(salary > 100000, "senior", "standard")
concat(first_name, " ", last_name)
year(hire_date)
```

## Date arithmetic

| Expression | Meaning |
|------------|---------|
| `date + n` / `date - n` | Add / subtract `n` **days** |
| `date1 - date2` | Number of **days** between two dates |
| `datetime + n` / `datetime - n` | Add / subtract `n` **seconds** |

```text
order_date + 30          # 30 days later
ship_date - order_date   # days to ship
```

## Type coercion

Values are coerced as needed — numeric strings become numbers in arithmetic,
booleans become `1` / `0`, and nulls pass through most operations. The resulting
column type is inferred (a division yields a float, `year(...)` an integer, and
so on); adjust it afterwards with `t`.

## Worked examples

```text
# Computed columns ( = )
revenue / units                         # price per unit
concat(city, ", ", country)             # combined label
if(score >= 60, "pass", "fail")         # bucket
year(created_at)                        # extract year

# Row select ( | then the expression )
|!=amount > 1000                        # high-value rows
|!=department == "Sales"                # one department
|!=region in ("North", "South")         # a set of regions
```

After an expression row-select, the matched rows are **selected** (not hidden).
Turn them into their own sheet with `"`, delete the rest with `d`, or copy them
with `yr`.
