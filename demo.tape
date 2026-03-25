# pixelbeat demo recording
# Pre-requisite: px daemon must already be running with music loaded
# Run: vhs demo.tape

Output assets/demo.gif
Output assets/demo.mp4

Set Shell "zsh"
Set FontSize 16
Set Width 1200
Set Height 600
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 50ms
Set CursorBlink false

# --- Scene 1: Cassette tape UI ---
Hide
Type "clear"
Enter
Show
Sleep 300ms

Type "px status --format '{cassette:70}'"
Enter
Sleep 4s

# --- Scene 2: Custom format ---
Hide
Type "clear"
Enter
Show
Sleep 300ms

Type "px status --format '{icon} {title:.35} {bar:20} {elapsed}/{duration}'"
Enter
Sleep 3s

# --- Scene 3: Spectrum ---
Hide
Type "clear"
Enter
Show
Sleep 300ms

Type "px status --format '{spectrum:50}'"
Enter
Sleep 3s

# --- Scene 4: TUI mode (hero moment) ---
Type "px tui"
Enter
Sleep 8s

# Exit TUI
Type "q"
Sleep 500ms

# --- Scene 5: Final cassette ---
Hide
Type "clear"
Enter
Show
Sleep 300ms

Type "px status"
Enter
Sleep 3s
