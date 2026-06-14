# JOIN — enrich orders with customer details (LEFT join on customer_id).
Output ".github/assets/join.gif"

Set Shell "zsh"
Set FontSize 15
Set Width 1200
Set Height 700
Set TypingSpeed 50ms
Set Padding 14

Hide
Type "export PATH=$PWD/target/release:$PATH"
Enter
Type "clear"
Enter
Show

Type "tuitab .github/assets/data/orders.csv"
Sleep 500ms
Enter
Sleep 1800ms

# Start the JOIN wizard
Type "J"
Sleep 1200ms
# Step 1: [Browse file...]
Enter
Sleep 900ms
# Step 1b: type the right-hand file path
Type ".github/assets/data/customers.csv"
Sleep 800ms
Enter
Sleep 1200ms
# Step 2: choose LEFT join
Type "j"
Sleep 700ms
Enter
Sleep 1000ms
# Step 3: left key = customer_id
Type "j"
Sleep 500ms
Type " "
Sleep 700ms
Enter
Sleep 1000ms
# Step 4: right key = customer_id (pre-selected)
Sleep 700ms
Enter
Sleep 3000ms

Type "q"
Sleep 700ms
Type "q"
Sleep 500ms
