# Charts — histogram then grouped bar (pin a category column).
Output ".github/assets/charts.gif"

Set Shell "zsh"
Set FontSize 15
Set Width 1200
Set Height 700
Set TypingSpeed 55ms
Set Padding 14

Hide
Type "export PATH=$PWD/target/release:$PATH"
Enter
Type "clear"
Enter
Show

Type "tuitab .github/assets/data/sales.csv"
Sleep 500ms
Enter
Sleep 1600ms

# Histogram of revenue (numeric column, no pin)
Type "llll"
Sleep 700ms
Type "V"
Sleep 2200ms
Escape
Sleep 800ms

# Pin region, then chart revenue grouped by region.
# Pinning floats region to the front, so step right past
# date/category/units to land back on revenue.
Type "hhh"
Sleep 500ms
Type "!"
Sleep 900ms
Type "llll"
Sleep 600ms
Type "V"
Sleep 1400ms
# aggregation popup -> confirm Sum
Enter
Sleep 2400ms
Type "l"
Sleep 600ms
Type "l"
Sleep 1200ms
Escape
Sleep 600ms

Type "q"
Sleep 500ms
