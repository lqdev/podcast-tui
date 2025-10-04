# Testing Command Auto-Completion

## Test Instructions

1. Run the application: `cargo run`

2. Press `:` or `M-x` (Alt+x) to open the command prompt

3. Test basic completion:
   - Type `q` and press Tab → should complete to `quit`
   - Type `h` and press Tab → should complete to `help`
   - Type `theme` and press Tab → should show theme options

4. Test contextual completion:
   - Type `theme ` (with space) and press Tab → should show theme names
   - Type `theme d` and press Tab → should complete to `theme dark`
   - Type `buffer ` and press Tab → should show buffer names
   - Type `switch-to-buffer ` and press Tab → should show buffer names

5. Test completion cycling:
   - Type partial command and press Tab multiple times to cycle through options

6. Test completion hints:
   - Look for `[completion]` hints showing what will be completed

## Expected Features

✅ Auto-completion for all commands
✅ Contextual completion (e.g., theme names after "theme ")
✅ Tab cycling through options
✅ Visual hints showing completion suggestions
✅ Dynamic updates as you type
✅ Case-insensitive matching

## Key Bindings for Testing

- `:` or `M-x` (Alt+x) → Open command prompt
- `Tab` → Complete/cycle completions
- `Enter` → Execute command
- `Ctrl+G` or `Esc` → Cancel command input
- `Ctrl+P/N` or `Up/Down` → Navigate command history