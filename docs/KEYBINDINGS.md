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
- `r` - Refresh selected podcast feed
- `Shift+R` - Refresh all podcast feeds
- `Ctrl+r` - Hard refresh selected podcast (re-parse all episodes)
- `Shift+A` - Import podcasts from OPML file or URL
- `Shift+E` - Export subscriptions to OPML file

### Episode Management
- `Enter` - Play selected episode (when playback is implemented in Sprint 4)
- `Shift+D` - Download selected episode
- `Shift+X` - Delete downloaded file for selected episode
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

#### Download Commands
- `delete-all-downloads` / `clean-downloads` - Delete ALL downloaded episodes

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

## Future Playback Controls (Sprint 4)

The following playback controls will be added when audio playback is implemented:

- `Space` - Play/pause
- `s` - Stop playback
- `[` / `]` - Seek backward/forward (30s)
- `+` / `-` - Volume up/down
- `n` / `p` - Next/previous episode in queue

---

**Last Updated**: October 2025  
**Version**: 1.0.0-mvp (in development)  
**Status**: Sprint 3 Complete
