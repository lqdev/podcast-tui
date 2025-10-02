# What's Next - Podcast TUI Development Roadmap

## ✅ Current Status

### Completed (Sprint 0-1):
- ✅ Development environment and project setup
- ✅ Storage abstraction with JSON implementation  
- ✅ Core data models (Podcast, Episode)
- ✅ UI framework with Ratatui and Emacs-style keybindings
- ✅ Buffer system (Help, Podcast List, Episode List)
- ✅ Minibuffer and Status bar components
- ✅ Theme system
- ✅ RSS feed parser with feed-rs
- ✅ **Subscription manager with add/remove/refresh** ← Just completed!

### Current Sprint: Sprint 2 - RSS and Podcasts (Week 3)

## 🎯 Immediate Next Steps (This Week)

### Day 5-6: Podcast UI (Priority 1)
The subscription manager is ready, but you need UI to use it!

**1. Add Subscription UI Flow** (Day 5)
- Create "Add Podcast" minibuffer command
- Implement feed URL input with validation
- Show subscription progress/feedback
- Display success/error messages

**2. Refresh Feed UI** (Day 5)
- Add "Refresh Feed" command for current podcast
- Add "Refresh All" command
- Show progress indicator during refresh
- Display count of new episodes found

**3. Delete Subscription UI** (Day 6)
- Add "Delete Subscription" command
- Implement confirmation dialog
- Update podcast list after deletion

**4. Enhanced Podcast List Buffer** (Day 6)
- Integrate with SubscriptionManager to load real data
- Add keybindings for:
  - `C-x a` - Add new subscription
  - `r` - Refresh current podcast
  - `R` - Refresh all podcasts
  - `d` - Delete subscription
- Show podcast metadata (episodes count, last updated)
- Implement proper error handling and user feedback

### Day 7: OPML Foundation
**OPML Import/Export** (Basic implementation)
- Parse OPML XML format
- Import multiple subscriptions from OPML file
- Export current subscriptions to OPML
- Add `M-x import-opml` and `M-x export-opml` commands

---

## 📋 Sprint 3: Episodes and Downloads (Week 4)

### Day 1-2: Episode Management
**Episode List Integration**
- Connect EpisodeList buffer to storage
- Load episodes for selected podcast
- Display episode metadata properly
- Mark episodes as new/played
- Sorting options (date, title, duration)

**Episode Detail View**
- Create dedicated episode detail buffer
- Show full episode information
- Display notes, chapters, transcript (if available)
- Episode status indicators

### Day 3-4: Download System
**HTTP Download Implementation**
- Create `DownloadManager` struct
- Implement FIFO download queue
- Support 2-3 concurrent downloads
- Progress tracking with tokio channels
- Resume interrupted downloads

**File Management**
- Organize downloads by podcast
- Filename sanitization and deduplication
- Storage location configuration
- Disk space checking

### Day 5-6: Download UI
**Download Commands and UI**
- `d` - Download selected episode
- `D` - Download all new episodes
- Download queue display
- Progress bar in status line
- Cancel download functionality

### Day 7: Integration
- Episode status integration (new → downloading → downloaded)
- Error handling for network failures
- Cleanup for old/played episodes
- Download limits per podcast

---

## 📋 Sprint 4: Playback System (Week 5)

### Day 1-2: Audio Backend
- Integrate rodio for audio playback
- Basic controls (play/pause/stop)
- Volume control
- Seek functionality (±30s)

### Day 3-4: Playback UI
- Now playing display
- Progress bar for current episode
- Playback controls in episode view
- Keyboard shortcuts (Space, +/-, Left/Right)

### Day 5-6: Advanced Features
- Chapter navigation
- Resume from last position
- Playback queue/autoplay next
- External player integration (mpv/vlc fallback)

### Day 7: Polish
- Playback error handling
- Save/restore playback position
- Integration with episode status

---

## 🔧 Quick Wins (Can do anytime)

### Code Quality
- [ ] Fix all clippy warnings
- [ ] Remove unused imports
- [ ] Add more unit tests for subscription manager
- [ ] Document public APIs with examples

### UI Polish
- [ ] Better error messages
- [ ] Loading indicators
- [ ] Help text improvements
- [ ] Keyboard shortcut hints

### Storage Improvements
- [ ] Add caching for frequently accessed data
- [ ] Implement proper error recovery
- [ ] Add data validation
- [ ] Consider compression for storage

---

## 🚀 How to Continue Development

### Test Current Features:
```bash
cd /workspaces/podcast-tui
cargo run
```

### Next Task to Implement:
**Start with the Podcast List UI integration** - this will make the subscription manager actually usable!

1. Open `src/ui/buffers/podcast_list.rs`
2. Add methods to load podcasts using `SubscriptionManager`
3. Implement the add/delete/refresh commands
4. Connect keybindings to the commands
5. Test with real RSS feeds

### Example Test Feeds:
```
- BBC World Service: https://podcasts.files.bbci.co.uk/p02nq0gn.rss
- The Daily: https://feeds.simplecast.com/54nAGcIl
- Reply All: https://feeds.megaphone.fm/replyall
```

### Testing Your Changes:
```bash
# Build and run
cargo run

# Run tests
cargo test

# Check for issues
cargo clippy
```

---

## 📝 MVP Checklist (by Sprint 7)

### Must Have:
- [x] RSS feed parsing
- [x] Subscription management (add/remove/refresh)
- [ ] Episode browsing
- [ ] Download management
- [ ] Audio playback (basic)
- [ ] OPML import/export
- [ ] Episode notes
- [ ] Persistent storage

### Nice to Have (Post-MVP):
- [ ] Search functionality
- [ ] Playlist management
- [ ] Statistics tracking
- [ ] Auto-cleanup
- [ ] Transcript display
- [ ] Chapter navigation
- [ ] Custom themes
- [ ] Multiple windows

---

## 💡 Development Tips

### Following Copilot Instructions:
1. **Always code against Storage trait** - Never directly use JsonStorage
2. **Use async/await** for I/O operations
3. **Proper error handling** - No unwrap() in user-facing code
4. **Buffer-based UI** - Follow Emacs paradigms
5. **Test as you go** - Write tests for business logic

### Common Patterns:
```rust
// Error handling
.await
.map_err(|e| SubscriptionError::Storage(e.to_string()))?

// Async operations
let result = tokio::spawn(async move {
    // Long operation
}).await??;

// UI updates
self.minibuffer.set_content(MinibufferContent::Message(msg));
```

### When Stuck:
1. Check the implementation plan (docs/IMPLEMENTATION_PLAN.md)
2. Look at similar existing code
3. Read the copilot instructions (.github/copilot-instructions.md)
4. Focus on MVP features first

---

**Current Priority: Implement Podcast List UI with real data integration**

Good luck! 🎉
