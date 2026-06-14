# Computed column — revenue per unit from an expression.
# The new column is inserted right after the cursor column, so
# we park the cursor on revenue first; all columns stay on screen.
Output ".github/assets/compute.gif"

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

# Park the cursor on revenue
Type "l"
Sleep 250ms
Type "l"
Sleep 250ms
Type "l"
Sleep 250ms
Type "l"
Sleep 700ms

# Add a computed column from an expression
Type "="
Sleep 900ms
Type "revenue / units"
Sleep 1300ms
Enter
Sleep 3000ms
