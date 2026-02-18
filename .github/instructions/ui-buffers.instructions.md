# Instructions for UI Buffers (`src/ui/buffers/`)

## Buffer Trait Requirements

Every buffer MUST implement the `Buffer` trait from `src/ui/buffers/mod.rs`:

```rust
pub trait Buffer: Any {
    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool);
    fn handle_key(&mut self, key: KeyEvent) -> Option<UIAction>;
    fn title(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

## Adding a New Buffer

1. Create `src/ui/buffers/<name>.rs` implementing `Buffer` trait
2. Add `pub mod <name>;` to `src/ui/buffers/mod.rs`
3. Add a typed getter in `BufferManager` (e.g., `get_<name>_buffer_mut_by_id`)
4. Register the buffer creation in `UIApp` where appropriate
5. Add keybinding in `src/ui/keybindings.rs` if needed (e.g., F-key shortcut)
6. Add the buffer to the help text in `src/ui/buffers/help.rs`

## Downcasting Buffers

Use safe downcasting via `as_any_mut()` — never use unsafe pointer casts:

```rust
// ✅ Correct
if let Some(buf) = buffer.as_any_mut().downcast_mut::<EpisodeListBuffer>() {
    buf.set_filter(filter);
}

// ❌ Never do this
let buf = unsafe { &mut *(buffer as *mut dyn Buffer as *mut EpisodeListBuffer) };
```

## Background Refresh Pattern

Buffers that load data asynchronously should use `tokio::spawn` to avoid blocking the UI:

```rust
// In UIApp, trigger a background refresh
tokio::spawn(async move {
    let data = storage.load_episodes(&podcast_id).await;
    let _ = tx.send(AppEvent::BufferDataRefreshed(BufferRefreshData::Episodes(data)));
});
```

## Buffer ID vs Display Name

- Buffer IDs are unique strings like `episode-list-{uuid}` or `help-{uuid}`  
- Use `BufferManager::find_buffer_id_by_name()` to resolve display names to IDs
- `switch_to_buffer()` takes a buffer ID, not a display name

## Current Buffers (12 total)

| Buffer | File | F-Key | Description |
|--------|------|-------|-------------|
| PodcastList | `podcast_list.rs` | F2 | Main podcast subscription list |
| EpisodeList | `episode_list.rs` | — | Episodes for a podcast (with filter support) |
| EpisodeDetail | `episode_detail.rs` | — | Single episode detail view |
| Downloads | `downloads.rs` | F4 | Active downloads progress |
| Help | `help.rs` | F1/F3 | Keybinding reference |
| BufferList | `buffer_list.rs` | Ctrl+b | Buffer switcher overlay |
| PlaylistList | `playlist_list.rs` | F7 | All playlists |
| PlaylistDetail | `playlist_detail.rs` | — | Single playlist view |
| PlaylistPicker | `playlist_picker.rs` | — | Add-to-playlist picker overlay |
| Sync | `sync.rs` | — | Device sync history |
| WhatsNew | `whats_new.rs` | — | Rolling new episodes across all podcasts |
