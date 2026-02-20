---
name: add-new-command
description: Add a new minibuffer command (e.g., :my-command) to the podcast-tui application. Covers command registration, argument parsing, tab completion, and help text.
---

# Skill: Add a New Minibuffer Command

## When to use
When you want users to be able to type `:my-command [args]` in the minibuffer.

## Steps

### 1. Identify where to handle the command
Commands are dispatched in `src/ui/app.rs` in the command execution section. Find the match block that handles string commands.

### 2. Add command handling

```rust
// In src/ui/app.rs command dispatch
"my-command" | "my-cmd" => {
    let arg = parts.get(1).map(|s| s.as_str()).unwrap_or("");
    self.handle_my_command(arg).await?;
}
```

### 3. Implement the handler
Add a method to `UIApp` or delegate to the appropriate manager:

```rust
async fn handle_my_command(&mut self, arg: &str) -> Result<()> {
    if arg.is_empty() {
        self.minibuffer.set_message("Usage: :my-command <value>");
        return Ok(());
    }
    // logic here
    self.minibuffer.set_message(&format!("Done: {}", arg));
    Ok(())
}
```

### 4. Add tab completion
In `src/ui/app.rs` or wherever tab completion candidates are built, add:

```rust
// Command completion candidates
"my-command",
"my-cmd",  // alias
```

### 5. Update help buffer
In `src/ui/buffers/help.rs`, add to the appropriate section:

```
:my-command <value>    Description of what it does
:my-cmd <value>        Alias for my-command
```

### 6. Update KEYBINDINGS.md
Add to `docs/KEYBINDINGS.md` in the relevant section:
```markdown
- `:my-command <value>` - Description of what it does
```

### 7. Add to CHANGELOG.md
Document the new command in the `[Unreleased]` section.

## Patterns to follow
- Use the `cleanup` / `clean-older-than` commands as a reference for commands with arguments and confirmation prompts
- Use `filter-status` as a reference for commands that update filter state
- Commands that modify data should confirm before acting (use `minibuffer.prompt()`)
- Provide a `:my-command-alias` short form for frequently used commands
