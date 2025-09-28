# Emacs-Style Keybindings

This document outlines the Emacs-inspired keybinding system for the Podcast TUI application.

## Navigation Keybindings

### Basic Movement
- `C-n` - Next line/item
- `C-p` - Previous line/item  
- `C-f` - Forward character/move right
- `C-b` - Backward character/move left
- `C-a` - Beginning of line/first item
- `C-e` - End of line/last item
- `M-<` - Beginning of buffer/first item
- `M->` - End of buffer/last item

### Buffer Management
- `C-x b` - Switch buffer (podcasts, episodes, playlists, etc.)
- `C-x 1` - Delete other windows
- `C-x 2` - Split window horizontally  
- `C-x 3` - Split window vertically
- `C-x o` - Switch to other window
- `C-x 0` - Delete current window

### Search and Commands
- `C-s` - Search forward
- `C-r` - Search backward
- `M-x` - Execute command by name
- `C-g` - Cancel current command
- `C-u` - Universal argument (repeat next command)

## Application-Specific Keybindings

### Podcast Management
- `a` - Add new podcast subscription
- `d` - Delete/unsubscribe podcast
- `r` - Refresh podcast feed
- `R` - Refresh all feeds
- `o` - Open podcast details

### Episode Management
- `RET` - Play selected episode
- `SPC` - Toggle play/pause
- `s` - Stop playback
- `n` - Next episode
- `p` - Previous episode
- `f` - Seek forward 30s
- `b` - Seek backward 30s
- `+` / `=` - Volume up
- `-` - Volume down
- `m` - Mark episode as played/unplayed
- `D` - Download episode
- `X` - Delete downloaded file
- `N` - Add/edit episode note

### Playlist Management
- `P` - Create new playlist
- `A` - Add episode to playlist
- `R` - Remove episode from playlist
- `M-n` - Move episode down in playlist
- `M-p` - Move episode up in playlist

### Filtering and Search
- `l` - Filter episodes (by status, date, etc.)
- `L` - Clear all filters
- `/` - Quick search
- `C-s` - Incremental search

### Help and Information
- `C-h ?` - Help for help
- `C-h k` - Describe key
- `C-h m` - Describe current mode
- `C-h b` - Show all keybindings
- `?` - Quick help overlay

### File Operations
- `C-x C-s` - Save configuration
- `C-x C-f` - Import OPML file
- `C-x C-w` - Export OPML file

## Minibuffer Commands

Commands that can be executed via `M-x`:

- `add-podcast` - Add new podcast subscription
- `import-opml` - Import OPML file
- `export-opml` - Export subscriptions to OPML
- `cleanup-episodes` - Clean up old downloaded episodes
- `show-statistics` - Display listening statistics
- `reload-config` - Reload configuration file
- `create-playlist` - Create new playlist
- `search-episodes` - Search across all episodes
- `filter-by-status` - Filter episodes by download/play status
- `filter-by-date` - Filter episodes by date range
- `quit` - Exit application

## Mode-Specific Behaviors

### Podcast List Mode
- Focus is on podcast subscriptions
- `RET` opens episode list for selected podcast
- `d` marks for deletion, `x` executes deletions
- `r` refreshes selected podcast

### Episode List Mode  
- Focus is on episodes of current podcast
- `RET` plays selected episode
- `D` downloads episode
- `N` adds note to episode

### Playlist Mode
- Focus is on playlist contents
- `RET` plays from selected position
- `A` adds current episode to playlist
- `R` removes selected episode

### Playback Mode
- Shows currently playing episode
- Enhanced playback controls available
- Chapter navigation if supported

## Configuration

All keybindings can be customized in the JSON configuration file:

```json
{
  "keybindings": {
    "global": {
      "quit": "C-x C-c",
      "help": "C-h ?",
      "search": "C-s"
    },
    "podcast_list": {
      "add": "a",
      "delete": "d",
      "refresh": "r"
    },
    "episode_list": {
      "play": "RET", 
      "download": "D",
      "note": "N"
    }
  }
}
```