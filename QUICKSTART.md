# Quick Reference - Podcast TUI

## TL;DR - What Works Right Now

### ✅ Working Features (Sprint 3 Complete)
- Subscribe to RSS podcast feeds
- Browse episodes with full metadata
- Download episodes (2-3 at a time)
- OPML import/export
- Intuitive keyboard shortcuts
- 4 color themes

### ❌ Not Yet Working
- Audio playback (coming in Sprint 4)
- Playlists (Sprint 5)
- Episode notes (Sprint 5)
- Statistics (Sprint 6)

## Installation Speed Run

### Windows x64
```powershell
# Needs: Rust + MSVC Build Tools
winget install Rustlang.Rustup
winget install Microsoft.VisualStudio.2022.BuildTools

git clone https://github.com/yourusername/podcast-tui.git
cd podcast-tui
cargo build --release
.\target\release\podcast-tui.exe
```

### Windows ARM64
```powershell
# Needs: Rust + LLVM/Clang
# See scripts/INSTALL-LLVM.md for LLVM setup
git clone https://github.com/yourusername/podcast-tui.git
cd podcast-tui
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
cargo build --release
.\target\release\podcast-tui.exe
```

### Linux
```bash
# Needs: Rust + build essentials
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install build-essential pkg-config libssl-dev # Ubuntu/Debian

git clone https://github.com/yourusername/podcast-tui.git
cd podcast-tui
cargo build --release
./target/release/podcast-tui
```

## Essential Keybindings

### Must Know
| Key | Action |
|-----|--------|
| `F1` or `?` | Help (press this first!) |
| `:` | Command prompt |
| `Esc` | Cancel |
| `q` or `F10` | Quit |

### Navigation
| Key | Action |
|-----|--------|
| `↓` / `↑` | Next/Previous item |
| `Enter` | Select/Activate |
| `Tab` | Next buffer |
| `Shift+Tab` | Previous buffer |
| `Ctrl+b` | Switch buffer |

### Podcast Management
| Key | Action |
|-----|--------|
| `a` | Add podcast |
| `d` | Delete podcast |
| `r` | Refresh feed |
| `R` | Refresh all |

### Episode Actions
| Key | Action |
|-----|--------|
| `Shift+D` | Download episode |
| `Shift+X` | Delete download |
| `Ctrl+x` | Delete ALL downloads |

## First 5 Minutes

1. **Start app**: `podcast-tui`
2. **Add podcast**: Press `a`, paste feed URL, press Enter
3. **Browse**: Use arrow keys (`↑`/`↓`) or `j`/`k`
4. **Download**: Press `Shift+D` on an episode
5. **Get help**: Press `F1` or `?` anytime

## Common Issues

### Build Fails
**Windows**: Need MSVC tools → `winget install Microsoft.VisualStudio.2022.BuildTools`  
**Windows ARM**: Need LLVM → See `scripts/INSTALL-LLVM.md`  
**Linux**: Need OpenSSL → `sudo apt install libssl-dev`

### Keys Don't Work
- Try arrow keys instead of `C-n`/`C-p`
- Check terminal emulator settings
- Use Windows Terminal on Windows

### Feed Won't Parse
- Verify URL is RSS/Atom feed
- Check internet connection
- Try URL in web browser first

## Good Test Feeds

```
The Changelog: https://changelog.com/podcast/feed
Syntax: https://feed.syntax.fm/rss
Reply All: https://feeds.megaphone.fm/replyall
```

## Configuration Location

**Linux**: `~/.config/podcast-tui/config.json`  
**Windows**: `%APPDATA%\podcast-tui\config.json`

## File Locations

**Linux**: `~/.local/share/podcast-tui/`  
**Windows**: `%LOCALAPPDATA%\podcast-tui\`

## More Help

- Detailed guide: [GETTING_STARTED.md](GETTING_STARTED.md)
- Full README: [README.md](README.md)
- Keybindings: [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md)
- Report issues: GitHub Issues

---

**Status**: Sprint 3 Complete (37.5% of MVP)  
**Next**: Sprint 4 - Audio Playback  
**Updated**: October 2025
