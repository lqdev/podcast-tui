# OPML Command Autocomplete Fix

## Issue
The `import-opml` and `export-opml` commands were not appearing in the minibuffer autocomplete suggestions when typing commands after pressing `:` (colon).

## Root Cause
The `get_available_commands()` function in `src/ui/app.rs` was missing the OPML commands from its list of available commands. This function is used to populate the autocomplete candidates for the command prompt.

## Solution
Added the OPML commands to the `get_available_commands()` function:

```rust
// OPML commands
"import-opml".to_string(),
"export-opml".to_string(),
```

## Files Changed
- `src/ui/app.rs` - Added OPML commands to the command list (lines 1308-1309)

## Testing
1. Build the application: `cargo build --release`
2. Run the application: `cargo run --release`
3. Press `:` to open the command prompt
4. Type `imp` and press `Tab` - should autocomplete to `import-opml`
5. Clear and type `exp` and press `Tab` - should autocomplete to `export-opml`

## Related Files
- `src/ui/app.rs` - Command execution handler (lines 1122-1149)
- `src/ui/components/minibuffer.rs` - Minibuffer component with autocomplete logic
- `src/ui/keybindings.rs` - Keybindings including `:` for command prompt (line 155-156 for Shift-A/E shortcuts)

## Notes
- The OPML commands were already properly implemented and working via direct command execution
- The commands were also documented in the help buffer
- The only issue was missing autocomplete support
- Both commands support optional arguments:
  - `import-opml <path>` - Import from file path or URL
  - `export-opml <path>` - Export to specified file path
  - Without arguments, both commands will prompt for the required information

## Architecture Adherence
This fix follows the project's guidelines:
- ✅ Maintains separation of concerns (UI completion vs command execution)
- ✅ Uses the existing command system infrastructure
- ✅ Follows Rust naming conventions
- ✅ Does not introduce breaking changes
- ✅ Preserves existing functionality
- ✅ Simple, focused fix for MVP delivery
