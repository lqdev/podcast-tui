# Keybinding Reference

This document covers the keybinding preset system and full binding reference for Podcast TUI.

## Presets

Three presets are available. Set one in `~/.config/podcast-tui/config.json`:

```json
{
  "keybindings": {
    "preset": "vim"
  }
}
```

| Preset | Description |
|--------|-------------|
| `default` | Arrow keys + j/k (up/down) + C-p/C-n. `h` opens help. |
| `vim` | hjkl navigation. `h` → move left (not help). Help is F1/`?`. |
| `emacs` | C-p/C-n navigation. j/k aliases removed. |

If `preset` is omitted or unrecognised, `default` is used.

## Auto-Generated Help

Press `F1`, `?`, or run `:help` to open `*Help: Keybindings*`. This buffer reflects whichever preset is active — changes to your config are shown automatically.

## Default Preset

### Navigation

| Key(s) | Action |
|--------|--------|
| `Up`, `k`, `C-p` | Move up |
| `Down`, `j`, `C-n` | Move down |
| `Left` | Move left |
| `Right` | Move right |
| `PgUp` | Page up |
| `PgDn` | Page down |
| `Home`, `g` | Jump to top |
| `End`, `Shift+G` | Jump to bottom |
| `Ctrl+Up` | Move episode up (playlist) |
| `Ctrl+Down` | Move episode down (playlist) |

### Buffer Management

| Key(s) | Action |
|--------|--------|
| `Tab`, `Ctrl+PgDn` | Next buffer |
| `Shift+Tab`, `BackTab`, `Ctrl+PgUp` | Previous buffer |
| `Ctrl+k` | Close buffer |
| `Ctrl+b` | Switch to buffer |
| `Ctrl+l` | List buffers |
| `F2` | Open podcast list |
| `F4` | Open downloads |
| `F7` | Open playlists |
| `F8` | Open sync |

### Application Controls

| Key(s) | Action |
|--------|--------|
| `F1`, `h`, `?`, `Shift+?` | Show help |
| `F5` | Refresh |
| `F6` | Clear filters |
| `F3`, `/` | Search |
| `:`, `Shift+:` | Command prompt |
| `Enter`, `Space` | Select |
| `Esc` | Cancel |
| `q`, `F10` | Quit |

### Podcast & Episode Actions

| Key(s) | Action |
|--------|--------|
| `a` | Add podcast |
| `d` | Delete podcast |
| `r` | Refresh podcast |
| `Shift+R` | Refresh all podcasts |
| `Ctrl+r` | Hard refresh podcast |
| `Shift+D` | Download episode |
| `X`, `Shift+X` | Delete downloaded episode |
| `Ctrl+x` | Delete all downloads |
| `m` | Mark played |
| `u` | Mark unplayed |
| `*` | Toggle favorite |
| `c` | Create playlist |
| `p` | Add to playlist |
| `Shift+A` | Import OPML |
| `Shift+E` | Export OPML |
| `s` | Sync to device |
| `[` | Previous tab |
| `]` | Next tab |

## Vim Preset

All default bindings apply except:

| Key | Action | Change |
|-----|--------|--------|
| `h` | Move left | Was: open help |
| `j` | Move down | Removes `C-n` alias |
| `k` | Move up | Removes `C-p` alias |
| `l` | Move right | (new binding) |
| `F1`, `?`, `Shift+?` | Show help | `h` removed from show_help |

## Emacs Preset

All default bindings apply except:

| Key | Action | Change |
|-----|--------|--------|
| `C-p` | Move up | Removes `k` alias |
| `C-n` | Move down | Removes `j` alias |

## User Overrides

Any field under `global` in `keybindings` overrides the preset for that action. Omitted fields use the preset's defaults. Empty arrays (`[]`) are treated as omitted (no-op):

```json
{
  "keybindings": {
    "preset": "vim",
    "global": {
      "quit": ["C-q", "F10"]
    }
  }
}
```

## Command Prompt

Press `:` (or `Shift+:`) to open the command prompt. Press `Tab` to autocomplete.

### Core Commands

- `quit` / `q` — Exit
- `help` — Open keybindings help buffer
- `theme <name>` — Change theme (`dark`, `light`, `high-contrast`, `solarized`)
- `switch-to-buffer <name>` — Switch to a named buffer
- `list-buffers` — Show buffer list
- `close-buffer` — Close current buffer

### Podcast Commands

- `add-podcast <url>` — Subscribe to a podcast
- `import-opml [path/url]` — Import from OPML
- `export-opml [path]` — Export to OPML

### Filter & Search Commands

- `search` — Search episodes by title
- `filter-status <status>` — Filter: `new`, `downloaded`, `played`, `downloading`, `failed`, `favorited`
- `filter-date <range>` — Filter by date: `today`, `12h`, `7d`, `2w`, `1m`
- `clear-filters` / `widen` — Remove all filters

### Download Commands

- `delete-all-downloads` — Delete all downloads (with confirmation)
- `clean-older-than <duration>` — Delete downloads older than duration (`12h`, `7d`, `2w`, `1m`)

### Sync Commands

- `sync [path]` — Sync to device
- `sync-dry-run [path]` — Preview sync without applying

### Playlist Commands

- `playlist-create [name]` — Create playlist
- `playlist-delete <name>` — Delete playlist
- `playlist-refresh` — Refresh `Today` playlist

## Minibuffer Input

| Key | Action |
|-----|--------|
| `Enter` | Submit |
| `Tab` | Autocomplete / cycle completions |
| `Esc` | Cancel |
| `Backspace` | Delete character |
| `Left` / `Ctrl+b` | Cursor left |
| `Right` / `Ctrl+f` | Cursor right |
| `Up` / `Ctrl+p` | Previous command in history |
| `Down` / `Ctrl+n` | Next command in history |

## Terminal Compatibility

Keybindings are designed to work reliably across terminals:

- **Windows Terminal** — Full support (recommended)
- **VS Code Terminal** — Full support
- **Linux terminals** (gnome-terminal, konsole, xterm) — Full support
- **macOS Terminal** — Full support

Some terminals intercept certain key combinations. If a binding is unresponsive, use the alternate or remap via `config.json`.

---
*Last Updated: June 2025 | Version: v1.9.0 | Maintainer: [@lqdev](https://github.com/lqdev)*
