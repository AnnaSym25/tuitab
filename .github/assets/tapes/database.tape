# Editing a SQLite table and writing it back: every statement is shown first.
# The database is rebuilt from inventory.sql on each run, in gitignored tmp/.
Output .github/assets/database.gif

Set Shell "zsh"
Set FontSize 15
Set Width 1200
Set Height 700
Set Padding 14
Set TypingSpeed 55ms

Hide
Type "export PATH=$PWD/target/release:$PATH"
Enter
Type "rm -rf tmp/db-demo && mkdir -p tmp/db-demo && sqlite3 tmp/db-demo/inventory.sqlite < .github/assets/data/inventory.sql && cd tmp/db-demo"
Enter
Type "clear"
Enter
Show

Type "tuitab inventory.sqlite"
Enter
Sleep 2200ms

# The first screen is the database itself: its tables and views.
Enter
Sleep 2200ms

# Edit a cell.  NULL is shown as NULL, and typed as \N.
Type "lle"
Sleep 700ms
Backspace 3
Type "150"
Enter
Sleep 1600ms

# Ctrl+S offers the file the table came from.
Ctrl+S
Sleep 1400ms
Enter
Sleep 2000ms

# Nothing is written yet: this is the SQL, and it runs only on Enter.
Sleep 2500ms
Enter
Sleep 2500ms
