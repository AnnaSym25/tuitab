# Hero demo — open, navigate, sort, chart, describe.
Output ".github/assets/demo.gif"

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
Sleep 600ms
Enter
Sleep 2s

# Move to the revenue column
Type "l"
Sleep 350ms
Type "l"
Sleep 350ms
Type "l"
Sleep 350ms
Type "l"
Sleep 800ms

# Sort descending by revenue
Type "]"
Sleep 1800ms

# Histogram of revenue
Type "V"
Sleep 2400ms
Type "l"
Sleep 500ms
Type "l"
Sleep 1200ms
Escape
Sleep 700ms

# Per-column statistics
Type "I"
Sleep 2800ms
Type "q"
Sleep 700ms

Type "q"
Sleep 500ms
