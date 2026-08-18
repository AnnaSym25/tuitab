# The new-row form: one field per column, checked against the column's type.
Output .github/assets/rowform.gif

Set Shell "zsh"
Set FontSize 15
Set Width 1200
Set Height 700
Set Padding 14
Set TypingSpeed 55ms

Hide
Type "export PATH=$PWD/target/release:$PATH"
Enter
Type "clear"
Enter
Show

Type "tuitab .github/assets/data/sales.csv"
Enter
Sleep 2000ms

# O builds a form out of the sheet: a field per column, labelled with its type.
Type "O"
Sleep 1800ms

Type "2024-04-02"
Sleep 400ms
Tab
Sleep 300ms
Type "North"
Sleep 400ms
Tab
Sleep 300ms
Type "Hardware"
Sleep 400ms
Tab
Sleep 300ms

# units is an integer column, so a word is refused with the reason,
# and Enter will not insert until it is fixed.
Type "many"
Sleep 2000ms
Backspace 4
Sleep 500ms
Type "16"
Sleep 800ms
Tab
Sleep 300ms
Type "11200"
Sleep 1000ms

Enter
Sleep 2500ms

# The new row lands at the end of the table.
Type "G"
Sleep 2000ms
