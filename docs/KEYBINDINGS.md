# Keybinding Reference

This document outlines the keybinding system for the Podcast TUI application.

## Navigation Keybindings

### Basic Movement
- `↑` / `Down Arrow` - Move up
- `↓` / `Up Arrow` - Move down
- `←` / `Left Arrow` - Move left
- `→` / `Right Arrow` - Move right
- `Page Up` - Scroll up by page
- `Page Down` - Scroll down by page
- `Home` - Jump to first item
- `End` - Jump to last item
- `Enter` - Select/activate current item
- `Space` - Select/activate current item

### Buffer Management
- `Tab` - Switch to next buffer
- `Shift+Tab` - Switch to previous buffer
- `Ctrl+Page Up` - Switch to previous buffer (alternative)
- `Ctrl+Page Down` - Switch to next buffer (alternative)
- `F2` - Switch to podcast list buffer
- `F3` - Switch to help buffer
- `F4` - Switch to downloads buffer
- `F5` - Refresh current buffer
- `F7` - Switch to playlists buffer
- `Ctrl+b` - Show buffer list / Switch buffer
- `Ctrl+k` - Close current buffer
- `Ctrl+l` - List all buffers

### Application Controls
- `F1` - Show help
- `h` - Show help
- `?` - Show help
- `:` - Open command prompt
- `Esc` - Cancel current operation / Hide minibuffer
- `q` - Quit application
- `F10` - Quit application

## Application-Specific Keybindings

### Podcast Management
- `a` - Add new podcast subscription
- `d` - Delete selected podcast (with confirmation)
- `c` - Create playlist
- `r` - Refresh selected podcast feed
- `Shift+R` - Refresh all podcast feeds
- `Ctrl+r` - Hard refresh selected podcast (re-parse all episodes)
- `Shift+A` - Import podcasts from OPML file or URL
- `Shift+E` - Export subscriptions to OPML file

### Episode Management
- `Enter` - Play selected episode (when playback is implemented in Sprint 4)
- `Shift+D` - Download selected episode
- `Shift+X` - Delete downloaded file for selected episode
- `p` - Add selected episode to a playlist
- `Ctrl+↑` - Move selected playlist episode up (playlist detail buffer)
- `Ctrl+↓` - Move selected playlist episode down (playlist detail buffer)
- `Ctrl+x` - Delete ALL downloaded episodes and clean up downloads folder

### Minibuffer Input (When Active)
- `Enter` - Submit input
- `Tab` - Auto-complete command or cycle through completions
- `Esc` - Cancel input
- `Backspace` - Delete previous character
- `Left Arrow` / `Ctrl+b` - Move cursor left
- `Right Arrow` / `Ctrl+f` - Move cursor right
- `Up Arrow` / `Ctrl+p` - Previous command in history
- `Down Arrow` / `Ctrl+n` - Next command in history

## Command Prompt (`:` or `M-x`)

### Available Commands

#### Core Commands
- `quit` / `q` - Exit application
- `help` / `h` - Show help buffer

#### Theme Commands
- `theme <name>` - Change color theme
  - `theme dark` - Dark theme (default)
  - `theme light` - Light theme
  - `theme high-contrast` - High contrast theme
  - `theme solarized` - Solarized theme

#### Buffer Commands
- `buffer` / `b` - List buffers or switch to specific buffer
- `switch-to-buffer` / `switch-buffer` - Switch to a specific buffer
- `list-buffers` / `buffers` - Show buffer list
- `close-buffer` / `kill-buffer` - Close current or specified buffer

#### Podcast Commands
- `add-podcast <url>` - Add new podcast subscription
- `import-opml [path/url]` - Import subscriptions from OPML file or URL
- `export-opml [path]` - Export subscriptions to OPML file

#### Download & Cleanup Commands
- `delete-all-downloads` / `clean-downloads` - Delete ALL downloaded episodes (with confirmation)
- `clean-older-than <duration>` / `cleanup <duration>` - Delete downloads older than duration
  - Duration formats: `12h` (hours), `7d` (days), `2w` (weeks), `1m` (months)
  - Default unit is days if no suffix (e.g., `30` = 30 days)
  - Prompts for confirmation before deleting
  - Auto-cleanup also runs on startup when `cleanup_after_days` is set in config (default: 30)

#### Sync Commands
- `sync [path]` / `sync-device [path]` - Sync podcasts + playlists to external device
- `sync-dry-run [path]` / `sync-preview [path]` - Preview sync changes without applying them

#### Playlist Commands
- `playlists` - Open playlists buffer
- `playlist-create [name]` / `playlist-new [name]` - Create playlist
- `playlist-delete <name>` - Delete playlist by name
- `playlist-refresh` - Refresh auto-generated `Today` playlist
- `playlist-sync` - Trigger standard device sync (podcasts + playlists)

#### Search & Filter Commands
- `search` - Search episodes by title (also `/` keybinding)
- `filter-status <status>` - Filter by status: `new`, `downloaded`, `played`, `downloading`, `failed`
- `filter-date <range>` - Filter by publish date: `today`, `12h`, `7d`, `2w`, `1m` (same syntax as cleanup)
- `clear-filters` / `widen` - Remove all active filters

> **Note**: Duration filtering (`filter-duration`) is deferred until episode duration
> data is populated from RSS feeds. See Design Decision #13 in `docs/SEARCH_AND_FILTER.md`.

### Auto-Completion Features

The command prompt supports intelligent auto-completion:

- **Tab Completion**: Press `Tab` to complete commands or cycle through options
- **Contextual Suggestions**: Get relevant suggestions based on what you're typing
  - `theme ` + Tab → shows available themes
  - `buffer ` + Tab → shows available buffer names
  - `switch-to-buffer ` + Tab → shows buffer names for switching
- **Visual Hints**: See completion candidates as you type
- **Dynamic Updates**: Completions update as you type
- **Case-Insensitive**: Matching works regardless of case

## Mode-Specific Behaviors

### Podcast List Buffer
- Shows all subscribed podcasts
- `Enter` on a podcast opens its episode list
- `a` adds a new podcast subscription
- `d` deletes the selected podcast (with confirmation)
- `r` refreshes the selected podcast feed
- `Shift+R` refreshes all podcast feeds

### Episode List Buffer  
- Shows episodes for a specific podcast
- `Enter` plays the selected episode (when playback is implemented)
- `Shift+D` downloads the selected episode
- `Shift+X` deletes the downloaded file for selected episode

### Downloads Buffer
- Shows all downloaded episodes across all podcasts
- Displays download progress for active downloads
- Shows file sizes and download status

### Playlists Buffer
- Shows all user playlists plus auto-generated `Today` playlist
- `Enter` opens playlist detail
- `c` creates a playlist
- `d` deletes selected playlist (with confirmation)
- `r` refreshes `Today` (from playlist list)
- `r` in playlist detail refreshes `Today` for auto playlists, or rebuilds files for user playlists

### Help Buffer
- Read-only buffer with help information
- Use standard navigation keys to scroll
- Press `q` or switch buffers to close

### Buffer List Buffer
- Shows all open buffers
- `Enter` on a buffer switches to it
- Navigate with arrow keys or vim-style keys

## Configuration

Keybindings are currently hardcoded for simplicity and reliability. Custom keybinding configuration may be added in a future release.

## Terminal Compatibility

These keybindings are designed to work reliably across different terminal emulators:

- **Windows Terminal** - Full support (recommended)
- **PowerShell** - Full support
- **cmd.exe** - Most keys work (some limitations)
- **VS Code Terminal** - Full support
- **Linux terminals** - Full support (gnome-terminal, konsole, xterm, etc.)
- **macOS Terminal** - Full support

**Note**: Some terminal emulators may intercept certain key combinations. If a keybinding doesn't work, try the alternative binding or adjust your terminal emulator settings.

## Future Playback Controls (Not Yet Implemented)

The following playback controls will be added when audio playback is implemented:

- `Space` - Play/pause
- `s` - Stop playback
- `[` / `]` - Seek backward/forward (30s)
- `+` / `-` - Volume up/down
- `n` / `p` - Next/previous episode in queue

---

**Last Updated**: February 2026  
**Version**: 1.6.0
