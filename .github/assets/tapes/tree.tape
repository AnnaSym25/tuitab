# JSON/YAML/TOML open as a table over the real document, not a flattened copy.
Output .github/assets/tree.gif

Set Shell "zsh"
Set FontSize 15
Set Width 1200
Set Height 700
Set Padding 14
Set TypingSpeed 55ms

Hide
Type "export PATH=$PWD/target/release:$PATH"
Enter
Type "rm -rf tmp/tree-demo && mkdir -p tmp/tree-demo && cd tmp/tree-demo && cp ../../.github/assets/data/config.toml ."
Enter
Type "clear"
Enter
Show

Type "tuitab config.toml"
Enter
Sleep 2200ms

# A document opens key/value: scalars, and the nested parts named by their shape.
Type "jjjjj"
Sleep 900ms

# Enter dives into the nested list — it is a table of its own.
Enter
Sleep 2200ms

# Edit a value in there; it goes back into the document, not into a copy.
Type "lle"
Sleep 700ms
Backspace
Type "9"
Enter
Sleep 1500ms

# Pop back out to the document.
Type "q"
Sleep 1500ms

# Converting is just a different extension.
Ctrl+S
Sleep 900ms
Backspace 20
Type "config.yaml"
Sleep 600ms
Enter
Sleep 2200ms

# Back out and look at what was written.
Type "q"
Sleep 900ms
Type "q"
Sleep 900ms
Type "y"
Sleep 1200ms
Type "cat config.yaml"
Enter
Sleep 3500ms
