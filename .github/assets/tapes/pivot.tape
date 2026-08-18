# Pivot — pin the index column (region), cursor on the pivot column
# (category), then type only the aggregation formula.
Output ".github/assets/pivot.gif"

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
Sleep 1800ms

# Pin region as the pivot index
Type "l"
Sleep 500ms
Type "!"
Sleep 1000ms

# Move cursor onto category (the column to spread across).
# Pinning shows region in the frozen block on the left but does
# not reorder the frame, so one step right lands on category.
Type "l"
Sleep 800ms

# Aggregation formula only
Type "W"
Sleep 1000ms
Type "sum(revenue)"
Sleep 1200ms
Enter
Sleep 3000ms

Type "q"
Sleep 700ms
Type "q"
Sleep 500ms
