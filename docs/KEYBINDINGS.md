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

## Startup Buffer

Choose which buffer opens when Podcast TUI launches. Add `ui.startup_buffer` to `~/.config/podcast-tui/config.json`:

```json
{
  "ui": {
    "startup_buffer": "whats-new"
  }
}
```

| Value | Buffer |
|-------|--------|
| `help` (default) | `*Help: Keybindings*` — auto-generated keybinding reference |
| `podcast-list` | Subscribed podcasts |
| `downloads` | Active and recent downloads |
| `sync` | Device sync history |
| `playlist-list` | Playlists |
| `whats-new` | Rolling new episodes across all podcasts |
| `now-playing` | Audio playback status |

Unknown values fall back to `help` and log a warning to stderr.

## Auto-Generated Help

Press `F1`, `?`, or run `:help` to open `*Help: Keybindings*`. This buffer reflects whichever preset is active for the current session. If you change `config.json` (including the `preset`), restart Podcast TUI to update the keybindings shown here.

## Quick-Reference Bar

Most buffers render a single contextual hint row just above the minibuffer listing the
buffer-specific keybindings most relevant to the current view (for example
`a add  d delete  r refresh` in the podcast list). Some hints are state-aware — the
episode list and What's New buffers only show `F6 clear filter` while a filter is active.
Buffers with their own richer in-pane hints (the Sync directory picker) and transient
overlays don't render the bar. Global buffer-switching function keys (`F2`, `F4`, `F7`, …)
are intentionally omitted to keep the bar focused on actions available right where you are,
though a few contextual function keys such as `F6 clear filter` still appear when relevant;
the full reference lives in the auto-generated help buffer above.

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
| `End`, `S-G` | Jump to bottom |
| `C-Up` | Move episode up (playlist) |
| `C-Down` | Move episode down (playlist) |

### Buffer Management

| Key(s) | Action |
|--------|--------|
| `Tab`, `C-PgDn` | Next buffer |
| `S-Tab`, `BackTab`, `S-BackTab`, `C-PgUp` | Previous buffer |
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
| `F1`, `h`, `?`, `S-?` | Show help |
| `F5` | Refresh |
| `F6` | Clear filters |
| `F3`, `/` | Search |
| `:`, `S-:` | Command prompt |
| `Enter`, `Space` | Select |
| `Esc` | Cancel |
| `q`, `F10` | Quit |

### Podcast & Episode Actions

| Key(s) | Action |
|--------|--------|
| `a` | Add podcast |
| `d` | Delete podcast |
| `r` | Refresh podcast |
| `S-R` | Refresh all podcasts |
| `C-r` | Hard refresh podcast |
| `S-D` | Download episode |
| `X`, `S-X` | Delete downloaded episode |
| `C-x` | Delete all downloads |
| `m` | Mark played |
| `u` | Mark unplayed |
| `*`, `S-*` | Toggle favorite |
| `c` | Create playlist |
| `p` | Add to playlist |
| `S-A` | Import OPML |
| `S-E` | Export OPML |
| `s` | Sync to device |
| `[` | Previous tab |
| `]` | Next tab |

### Audio Playback

| Key(s) | Action |
|--------|--------|
| `S-P` | Toggle play / pause |
| `⏯` (media key) | Toggle play / pause |
| `⏵` (media key) | Toggle play / pause |
| `S-Enter` | Play selected downloaded episode |
| `C-Left` | Seek backward 10 s |
| `C-Right` | Seek forward 10 s |
| `+` / `=` | Volume up |
| `-` | Volume down |
| `F9` | Open now playing buffer |

## Vim Preset

All default bindings apply except:

| Key | Action | Change |
|-----|--------|--------|
| `h` | Move left | Was: open help |
| `j` | Move down | Removes `C-n` alias |
| `k` | Move up | Removes `C-p` alias |
| `l` | Move right | (new binding) |
| `F1`, `?`, `S-?` | Show help | `h` removed from show_help |

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
- `theme <name>` — Change theme. Built-in: `dark`, `light`, `high-contrast`, `solarized`. Community (bundled): `catppuccin-mocha`, `dracula`, `nord`, `gruvbox-dark`, `tokyo-night`. Also accepts any user TOML theme name.
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
- `set-device-profile [name]` — Switch the active device profile and persist the change to `config.json`. Tab-completes against `device_profiles[].name` from `config.json`. Empty argument clears the active profile. If the profile has a `sync_path`, the active sync target is set to it (overriding any manual pick; an unavailable path still applies with a warning). If the save fails an error is shown but the in-memory switch still applies for the current session. See [`docs/DEVICE_PROFILES.md`](DEVICE_PROFILES.md).

### Storage Commands

- `cache-rebuild` — Discard the in-memory storage cache and rebuild it from disk. If `storage.cache_enabled` is `true` (the default), the rebuilt index is then flushed atomically to `cache_index.json`; if caching is disabled, the snapshot is rebuilt in memory but no on-disk index is written. Use this if the cache appears stale (e.g. after editing data files outside the app). The on-disk JSON files are always the source of truth, so this is a safe escape hatch.

### Playlist Commands

- `playlist-create [name]` — Create playlist
- `playlist-delete <name>` — Delete playlist
- `playlist-refresh` — Refresh `Today` playlist
- `smart-playlist <name>` — Create a smart playlist (auto-generated based on rules)

### Discovery Commands

- `discover <query>` — Search PodcastIndex for podcasts matching `query`
- `trending` — Browse currently trending podcasts

### Tagging Commands

- `tag <tag>` — Tag the selected podcast
- `untag <tag>` — Remove a tag from the selected podcast
- `filter-tag <tag>` — Filter the podcast list to show only podcasts with this tag

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
*Last Updated: February 2026 | Version: v1.12.0 | Maintainer: [@lqdev](https://github.com/lqdev)*
