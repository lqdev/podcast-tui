# Colon (:) Keybinding Fix

## Problem
The `:` keybinding to enter command mode wasn't working. When users pressed the `:` key, nothing happened and they couldn't enter commands.

## Root Cause
The issue was in the keybinding registration in `src/ui/keybindings.rs`. The `:` character is typically produced by pressing `Shift + ;` on most keyboards. However, different terminals and platforms report this key event differently:

- Some terminals report it as `KeyCode::Char(':')` with `KeyModifiers::NONE`
- Other terminals report it as `KeyCode::Char(':')` with `KeyModifiers::SHIFT`

The keybinding was only registered with `KeyModifiers::NONE`, which meant it wouldn't work on terminals that include the SHIFT modifier when reporting the `:` character.

## Solution
Added two keybindings for the `:` character to handle both cases:

```rust
// Bind ':' without modifiers (crossterm handles the shift automatically for the char)
self.bind_key(KeyChord::none(KeyCode::Char(':')), UIAction::PromptCommand);
// Also bind with shift modifier in case some terminals report it that way
self.bind_key(KeyChord::shift(KeyCode::Char(':')), UIAction::PromptCommand);
```

This ensures the keybinding works consistently across:
- Windows Terminal
- PowerShell
- CMD
- VS Code integrated terminal
- Various Linux/Unix terminals
- macOS Terminal

## Testing
To test the fix:

1. Build the application: `cargo build --release`
2. Run the application: `cargo run --release`
3. Press the `:` key
4. The minibuffer should show the command prompt: `M-x █`
5. You can now type commands and press Enter to execute them

## Related Files
- `src/ui/keybindings.rs` - Contains the keybinding registration
- `src/ui/app.rs` - Handles the `UIAction::PromptCommand` action
- `src/ui/components/minibuffer.rs` - Implements the command input interface

## Additional Notes
This is a common issue with character-based keybindings that require modifier keys. The fix follows the pattern already established in the codebase for other special characters like `?` (which also requires Shift on most keyboards).
