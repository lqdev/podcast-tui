# Bug Fix: OPML Import URL Handling

## Issue
When trying to import an OPML file from a URL (e.g., `https://www.lqdev.me/collections/podroll/index.opml`), the application was treating it as a podcast feed URL and trying to subscribe to it directly, instead of importing it as an OPML file.

## Root Cause
The bug was in the `handle_minibuffer_input` method. The flow was:

1. User presses `Shift+A` → Shows minibuffer with prompt "Import OPML from (file path or URL): "
2. User types a URL like `https://example.com/file.opml`
3. User presses Enter
4. `handle_minibuffer_key` receives Enter key
5. Calls `self.minibuffer.submit()` which:
   - Extracts the input
   - Calls `self.hide()` which **clears the minibuffer content**
   - Returns the input
6. Calls `handle_minibuffer_input(input)`
7. But `current_prompt()` now returns `None` because content was cleared!
8. Falls through to the URL check: "starts with https://" → tries to add as podcast

**The prompt context was lost because `submit()` cleared it before we could check it!**

## Solution
Changed the flow to capture the prompt context **before** calling `submit()`:

```rust
// Old (broken):
(KeyCode::Enter, _) => {
    if let Some(input) = self.minibuffer.submit() {
        self.handle_minibuffer_input(input);  // Prompt already cleared!
    }
}

// New (fixed):
(KeyCode::Enter, _) => {
    // Get the prompt BEFORE submit() clears it
    let prompt = self.minibuffer.current_prompt();
    if let Some(input) = self.minibuffer.submit() {
        self.handle_minibuffer_input_with_context(input, prompt);
    }
}
```

## Changes Made

### 1. `src/ui/app.rs`
- Modified Enter key handler to capture prompt before submit
- Created new `handle_minibuffer_input_with_context()` method
- Refactored existing logic into `handle_minibuffer_input_legacy()`
- Kept `handle_minibuffer_input()` for backward compatibility

### 2. Logic Flow
Now the prompt context is checked FIRST, before falling back to URL detection:

```rust
fn handle_minibuffer_input_with_context(&mut self, input: String, prompt_context: Option<String>) {
    // Check context from prompt FIRST (before checking for URLs)
    if let Some(prompt) = &prompt_context {
        if prompt.starts_with("Import OPML from") {
            // This is an OPML import
            self.trigger_async_opml_import(input.to_string());
            return;
        }
        // ... other context checks
    }
    
    // Only if no prompt context, check if it's a URL
    // ...
}
```

## Testing

### Before Fix
```
Input: https://www.lqdev.me/collections/podroll/index.opml
Result: "Failed to subscribe to https://www.lqdev.me/collections/podroll/index.opml"
Reason: Treated as podcast feed URL
```

### After Fix
```
Input: https://www.lqdev.me/collections/podroll/index.opml  
Result: OPML import starts correctly
Reason: Prompt context preserved, recognized as OPML import
```

## Impact
- ✅ Fixes OPML import from URLs
- ✅ Maintains backward compatibility
- ✅ No changes to OPML export functionality
- ✅ No changes to other minibuffer operations

## Build Status
✅ Compiles successfully with only pre-existing warnings

---

**Date**: October 6, 2025  
**Status**: Fixed and tested
