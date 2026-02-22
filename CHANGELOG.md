# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **`:theme` command now visually updates all open buffers and the minibuffer** — February 2026
  - Previously `:theme <name>` only updated the status bar and `self.theme`; all open buffers (podcast list, episode list, downloads, etc.) and the minibuffer kept the old theme's colours until the user restarted the app.
  - Fixed by calling `BufferManager::set_theme_all()` and `Minibuffer::set_theme()` from `set_theme_direct()`.
  - Six buffers (podcast list, episode list, episode detail, What's New, help, now playing) had inherent `set_theme()` methods but did not override the `Buffer` trait's `set_theme()`, so theme changes through `dyn Buffer` hit the no-op default. Added trait overrides for all six.
  - Closes [#167](https://github.com/lqdev/podcast-tui/issues/167).

- **`PlayEpisode` now has a default keybinding: `S-Enter` (Shift+Enter)** — audio playback was completely unreachable with default config — February 2026
  - `PlayEpisode` had no default keybinding (`play_episode: vec![]` in both `default_preset()` and `Config::default()`), making it literally impossible to start playing an episode without editing `config.json`. `Enter` opens the episode detail buffer; `p` adds to playlist; `S-P` only toggles between playing/paused (no effect when stopped). No `:play` minibuffer command exists.
  - Fixed by binding `S-Enter` (Shift+Enter) to `PlayEpisode` in `setup_default_bindings()` and setting `play_episode: ["S-Enter"]` in `default_preset()`. `Enter` (plain) is unaffected and still opens episode detail.
  - The existing two-phase dispatch is preserved: the keybinding stores nil placeholder IDs; the episode list buffer's `handle_action()` replaces them with real episode data before sending `AudioCommand::Play` to the audio backend.
  - User overrides remain fully supported: setting `"play_episode": ["F11"]` in `config.json` replaces `S-Enter` with `F11` (the override clears the default). Setting `"play_episode": []` keeps the `S-Enter` default.
  - Closes [#171](https://github.com/lqdev/podcast-tui/issues/171).

- **`TogglePlayPause` (`S-P`) now shows an error message when audio is unavailable** — previously silently no-oped when `audio_command_tx` is `None` (audio hardware init failed). Now mirrors `PlayEpisode`'s existing error pattern and shows "Audio playback not available on this system". Closes [#171](https://github.com/lqdev/podcast-tui/issues/171).

- **`:tag`/`:untag`/`:filter-tag` commands now work from any buffer — February 2026**
  - All three commands previously routed the action through `current_buffer_mut()`, which meant they silently did nothing when any buffer other than the podcast list had focus (episode list, downloads, help, etc.). Each non-podcast buffer has a `_ => UIAction::None` catch-all that swallowed the action without any feedback.
  - Fixed by routing all three commands directly to `get_podcast_list_buffer_mut()` — the same typed getter already used correctly by `:tags`.
  - `:tag <name>` now adds a tag to the selected podcast regardless of which buffer is active; shows a clear error when no podcast is selected (instead of silence).
  - `:untag <name>` now removes a tag regardless of active buffer; shows a clear error when no podcast is selected.
  - `:filter-tag <tag>` now filters the podcast list regardless of active buffer; automatically switches focus to the podcast list buffer so the user can see the result.
  - Closes [#168](https://github.com/lqdev/podcast-tui/issues/168).
  - 5 unit tests: `test_tag_command_routes_to_podcast_list_not_active_buffer`, `test_untag_command_routes_to_podcast_list_not_active_buffer`, `test_filter_tag_routes_to_podcast_list_and_switches_buffer`, `test_tag_command_shows_error_when_no_podcast_selected`, `test_untag_command_shows_error_when_no_podcast_selected`

### Added

- **Audio integration tests** — 7 integration tests for `AudioManager` end-to-end API (`tests/test_audio.rs`). 5 CI-safe tests use a platform-appropriate external player (`echo` on Unix, `cmd` on Windows); 2 hardware-dependent tests gated with `#[ignore]`. Includes `write_test_wav()` helper for dynamic WAV generation and `wait_for_event()` async helper with timeout+predicate matching. Closes [#143](https://github.com/lqdev/podcast-tui/issues/143). Part of [#142](https://github.com/lqdev/podcast-tui/issues/142).

- **Wire AudioManager into App event loop** — completes the playback integration pipeline so audio commands flow from key presses to the speaker. `App::run()` now spawns `AudioManager`, wires the command sender into `UIApp`, and passes the status watch receiver to `UIApp::run()`. A dedicated `select!` branch triggers re-renders on every `AudioManager` status tick. `UIApp` gains an `audio_command_tx: Option<…>` field and a `set_audio_command_tx()` setter; constructors unchanged (no test breakage). All 7 playback `UIAction` stubs are replaced with real `AudioCommand` dispatch; all 4 `AppEvent` playback stubs are replaced with real business logic. Closes [#141](https://github.com/lqdev/podcast-tui/issues/141). Part of [#137](https://github.com/lqdev/podcast-tui/issues/137).
  - `src/audio/manager.rs`: added `pub fn command_tx()` to expose a cloneable command sender
  - `src/app.rs`: `App::run()` creates `AudioManager`, extracts `audio_command_tx` + `playback_status_rx`, sets both on `UIApp`; graceful `eprintln!` fallback when audio init fails (app still starts)
  - `src/ui/app.rs`:
    - `UIApp::run()` accepts `playback_status_rx: Option<watch::Receiver<PlaybackStatus>>`; wires it to the NowPlaying buffer via `BufferManager::set_now_playing_status_rx()`
    - `select!` event loop branch: re-renders on `AudioManager` status change
    - `handle_action`: `PlayEpisode` → `AudioCommand::Play`; `TogglePlayPause` → `AudioCommand::TogglePlayPause` (guarded when minibuffer is in input mode); `StopPlayback` → `AudioCommand::Stop`; `SeekForward`/`SeekBackward` → `AudioCommand::SeekForward/Backward(SEEK_STEP_SECS)`; `VolumeUp`/`VolumeDown` → `AudioCommand::VolumeUp/VolumeDown`
    - `handle_app_event`: `PlaybackStarted` → loads episode + podcast from storage, calls `set_now_playing_info`, shows "Now playing…"; `PlaybackStopped` → shows message; `TrackEnded` → calls `episode.update_position(duration)` or `episode.mark_played()`, saves, triggers buffer refresh; `PlaybackError` → shows error
  - `src/ui/buffers/now_playing.rs`: added `episode_title()` and `podcast_name()` public getters
  - 8 unit tests: `test_handle_action_toggle_play_pause_sends_command_when_audio_available`, `test_handle_action_toggle_play_pause_blocked_in_minibuffer_input_mode`, `test_handle_action_stop_playback_sends_stop_command`, `test_handle_action_volume_up_sends_volume_up_command`, `test_handle_app_event_track_ended_marks_episode_played_and_persists`, `test_handle_app_event_track_ended_marks_played_when_no_duration`, `test_handle_app_event_playback_started_sets_now_playing_info`, `test_handle_app_event_playback_error_shows_error_message`


  - New `src/ui/buffers/now_playing.rs`: `NowPlayingBuffer` struct, `progress_ratio_and_label()` helper (reuses `utils::time::format_duration`)
  - `can_close()` returns `false` — persistent buffer, not closeable with `C-k`
  - `set_status_rx()` method: replaces watch receiver when AudioManager is wired in ([#141](https://github.com/lqdev/podcast-tui/issues/141))
  - `set_now_playing_info()` method: sets episode title and podcast name from `AppEvent::PlaybackStarted` (wired in [#141](https://github.com/lqdev/podcast-tui/issues/141))
  - `BufferManager` gains: `create_now_playing_buffer()`, `get_now_playing_buffer_mut()`, `set_now_playing_status_rx()`, `set_now_playing_info()`
  - Help buffer updated with PLAYBACK CONTROLS and NOW PLAYING BUFFER sections
  - 11 unit tests: `test_now_playing_buffer_title_returns_now_playing`, `test_now_playing_buffer_initial_state_is_stopped`, `test_now_playing_buffer_cannot_be_closed`, `test_now_playing_buffer_handle_action_returns_none`, `test_now_playing_buffer_formats_duration_correctly`, `test_now_playing_buffer_progress_ratio_clamped`, `test_now_playing_buffer_progress_ratio_zero_for_no_data`, `test_now_playing_buffer_progress_ratio_normal`, `test_now_playing_buffer_status_rx_update`, `test_now_playing_buffer_set_now_playing_info`, `test_now_playing_buffer_info_cleared_when_stopped`

- **Playback keybindings** — wires playback `UIAction` variants to keyboard shortcuts and makes them fully user-configurable. Closes [#139](https://github.com/lqdev/podcast-tui/issues/139). Part of [#137](https://github.com/lqdev/podcast-tui/issues/137).
  - Default bindings (no existing key displaced): `S-P` (Shift+P) → TogglePlayPause, `C-Left` → SeekBackward, `C-Right` → SeekForward, `+`/`=` → VolumeUp, `-` → VolumeDown, `F9` → SwitchBuffer("now-playing")
  - `PlayEpisode` is intentionally unbound by default (`p` is AddToPlaylist); configure via `play_episode` in `config.json`
  - 7 new configurable `GlobalKeys` fields in `src/config.rs`: `toggle_play_pause`, `play_episode`, `seek_backward`, `seek_forward`, `volume_up`, `volume_down`, `open_now_playing`
  - 6 unit tests: `test_default_bindings_shift_p_toggles_play_pause`, `test_from_config_media_keys_survive_preset_application`, `test_default_bindings_include_seek_keys`, `test_default_bindings_include_volume_keys`, `test_config_overrides_toggle_play_pause`, `test_config_overrides_play_episode`

- **Playback UIActions and AppEvents** — adds the full vocabulary for playback interactions between the UI layer, keybindings, and audio backend. Closes [#138](https://github.com/lqdev/podcast-tui/issues/138). Part of [#137](https://github.com/lqdev/podcast-tui/issues/137).
  - 7 new `UIAction` variants in `src/ui/mod.rs`: `PlayEpisode { podcast_id, episode_id, path }`, `TogglePlayPause`, `StopPlayback`, `SeekForward`, `SeekBackward`, `VolumeUp`, `VolumeDown`
  - `PlayEpisode` guard in `src/ui/buffers/episode_list.rs`: only fires when `episode.local_path.is_some()`; shows `ShowError` for episodes that are not yet downloaded
  - Stub match arms for all 7 variants in `app.rs handle_action()` — actual AudioManager dispatch wired in [#141](https://github.com/lqdev/podcast-tui/issues/141)
  - Note: `AppEvent` audio variants (`PlaybackStarted`, `PlaybackStopped`, `TrackEnded`, `PlaybackError`) were already added in [#136](https://github.com/lqdev/podcast-tui/issues/136)
  - 2 unit tests: `test_play_episode_action_requires_local_path`, `test_play_episode_action_contains_correct_path`

- **AudioManager — threaded playback coordinator** — creates and owns a `PlaybackBackend` on a dedicated `std::thread` (not `tokio::spawn`) to avoid cpal/rodio deadlock with the async runtime. Communicates with the UI via three channels: `mpsc::UnboundedSender<AudioCommand>` (UI → manager), `watch::Receiver<PlaybackStatus>` (continuous ~4 Hz state: position, volume, state), and `mpsc::UnboundedSender<AppEvent>` (one-shot events: `PlaybackStarted`, `PlaybackStopped`, `TrackEnded`, `PlaybackError`). Backend selection: `config.external_player` set → `ExternalPlayerBackend`; otherwise try `RodioBackend`, fall back to `ExternalPlayerBackend::detect()`, return original rodio error if both fail. Volume tracked in manager (0.0–1.0; `VolumeUp`/`VolumeDown` use `VOLUME_STEP = 0.05`). Thread exits cleanly when command channel is dropped. Closes [#136](https://github.com/lqdev/podcast-tui/issues/136). Part of [#132](https://github.com/lqdev/podcast-tui/issues/132).
  - New `src/audio/manager.rs`: `AudioManager` struct, `run_loop()`, `process_command()`, `create_backend()`
  - New `AppEvent` variants in `src/ui/events.rs`: `PlaybackStarted`, `PlaybackStopped`, `TrackEnded`, `PlaybackError`
  - 17 unit tests: mock `PlaybackBackend` (no audio hardware required), command routing, event firing, volume clamping, channel lifecycle

- **ExternalPlayerBackend — fallback audio via system media player** — implements `PlaybackBackend` by spawning `mpv`, `vlc`, or `ffplay` as a subprocess. Used when `config.audio.external_player` is set, or as a fallback when the native Rodio backend cannot initialize (no ALSA device, WSL2, containers). Auto-detects players in priority order: mpv → vlc → ffplay. Drop-safe: running child processes are killed when the backend is dropped. Closes [#135](https://github.com/lqdev/podcast-tui/issues/135). Part of [#132](https://github.com/lqdev/podcast-tui/issues/132).
  - New `src/audio/external.rs`: `ExternalPlayerBackend` struct, `build_player_args()` helper (correct headless flags per player), `detect()` / `detect_from_candidates()` auto-detection, `Drop` impl
  - `AudioError::Unsupported(String)` variant added to `src/audio/mod.rs`
  - Limitations: `pause()` / `resume()` are no-ops; `seek()` returns `AudioError::Unsupported`; `position()` / `duration()` return `None`

- **RodioBackend — native cross-platform audio playback via rodio** — implements `PlaybackBackend` using rodio 0.21 for native audio output (WASAPI on Windows, ALSA on Linux, CoreAudio on macOS). Full playback control: play/pause/resume/stop, seek via `Sink::try_seek()`, volume control (clamped to 0.0–1.0), position tracking via `Sink::get_pos()` (updated every ~5 ms by the audio thread), duration from decoder headers. Supports MP3/AAC/OGG/FLAC/WAV via `Decoder::try_from(file)` with file byte_len for accurate seeking and duration. `stop()` uses `Sink::clear()` (synchronous) so callers see `is_stopped() == true` immediately. Closes [#134](https://github.com/lqdev/podcast-tui/issues/134). Part of [#132](https://github.com/lqdev/podcast-tui/issues/132).
  - New `src/audio/rodio_backend.rs`: `RodioBackend` struct initialised via `OutputStreamBuilder::open_default_stream()`; `OutputStream` held for lifetime of backend (drop = silence)
  - `play()` creates a fresh `Sink` per track; old sink dropped cleanly; `Decoder::try_from(File)` for byte_len/seeking/duration
  - `seek()` via `Sink::try_seek()`, `position()` via `Sink::get_pos()`, `duration()` from decoder headers
  - `is_playing()` / `is_paused()` / `is_stopped()` delegate to `Sink::empty()` and `Sink::is_paused()`
  - Tests: 16 unit tests covering all acceptance criteria; device-aware skip guards for headless CI

- **PodcastIndex API integration for discovery** — new `:discover <query>` and `:trending` minibuffer commands search the [PodcastIndex.org](https://podcastindex.org/) API and display results in a dedicated `*Discover: …*` buffer. Press Enter on any result to subscribe immediately. API credentials are configured via `discovery.podcastindex_api_key` and `discovery.podcastindex_api_secret` in `config.json` (free credentials at <https://api.podcastindex.org/>). Auth uses a timestamp-based SHA-1 hash (per PodcastIndex spec). Closes [#110](https://github.com/lqdev/podcast-tui/issues/110).
  - New `src/podcast/discovery.rs`: `PodcastIndexClient`, `PodcastSearchResult`, `DiscoveryError`; `search()` and `trending()` async methods
  - New `src/ui/buffers/discovery.rs`: `DiscoveryBuffer` implementing `Buffer` trait with arrow-key navigation, description preview, and subscribe-on-enter
  - New `DiscoveryConfig` section in `src/config.rs` (`podcastindex_api_key`, `podcastindex_api_secret`; `#[serde(default)]` for backward compatibility)
  - New `AppEvent::DiscoveryResultsLoaded` / `DiscoveryLoadFailed` in `src/ui/events.rs`
  - New `UIAction::SubscribeFromDiscovery { feed_url }` in `src/ui/mod.rs`
  - `sha1 = "0.10"` added to `Cargo.toml` for auth header computation
  - Discovery constants added to `src/constants.rs`: `PODCASTINDEX_API_BASE_URL`, `DEFAULT_TRENDING_COUNT`, `DISCOVERY_REQUEST_TIMEOUT`, `MAX_SEARCH_RESULTS`
  - Help text updated with discovery command documentation
  - Tests: 9 unit tests in `discovery.rs` (SHA-1 vectors, URL encoding, error handling, client construction); 12 unit tests in `discovery.rs` buffer (navigation, state transitions, subscribe action)

- **Smart playlists (dynamic filter rules)** — new `:smart-playlist <name> [--filter <spec>...] [--sort <field>] [--limit <n>]` minibuffer command creates playlists that auto-populate based on filter criteria evaluated dynamically each time the playlist is opened. Filter specs: `downloaded`, `favorited`, `played`, `unplayed`, `tag:<name>`, `podcast:<id>`, `newer-than:<days>`; multiple `--filter` flags are AND-ed. Sort options: `date-desc` (default), `date-asc`, `title-asc`, `title-desc`. Smart playlists are marked with a `⚡` prefix in the playlist list and detail buffer. Reorder and remove-episode actions are blocked with a descriptive message for smart playlists. Closes [#109](https://github.com/lqdev/podcast-tui/issues/109).
  - New types: `SmartFilter`, `SmartSort`, `SmartSortField`, `SmartSortDirection`, `SmartPlaylistRule` in `src/playlist/models.rs`
  - `Playlist::is_smart()` convenience predicate; `smart_rules` field is `#[serde(default)]` for backward compatibility
  - `PlaylistManager::create_smart_playlist()` in `src/playlist/manager.rs`
  - Tests added: 10 unit tests covering filter evaluation, sort, limit, AND logic, and serde round-trip

- **Bundled community themes** — five popular community colour schemes are now shipped with the binary, available immediately via `:theme <name>` with no file setup required: `catppuccin-mocha`, `dracula`, `nord`, `gruvbox-dark`, `tokyo-night`. Each theme uses the official palette from its upstream project. Community themes are also valid `extends` parents in user `.toml` theme files (e.g. `extends = "catppuccin-mocha"`). Theme files live in `assets/themes/` and are embedded at compile time via `include_str!`. Closes [#105](https://github.com/lqdev/podcast-tui/issues/105).
  - Tests added: `parse_theme_str` unit tests, palette spot-checks for all 5 themes, `get_bundled()` coverage, `list_names()` coverage, extends-from-community-theme round-trip.

- **Theme inheritance (`resolve_theme`)** — new `resolve_theme(path, registry)` public API in `src/ui/theme_loader.rs` resolves `extends` fields using the `ThemeRegistry`, enforcing single-level bundled-only inheritance. New `Theme::color_scheme()` getter and `Theme::from_color_scheme()` constructor support programmatic theme construction. `ThemeRegistry::get_bundled()` restricts parent lookup to built-in bundled themes, preventing user-defined themes from acting as parents (avoids circular dependencies). Closes [#104](https://github.com/lqdev/podcast-tui/issues/104).
  - Tests added: unit tests covering override, fallback to dark base, all bundled parents, unknown parent error, user-theme-as-parent rejection, `color_scheme()` getter, and `from_color_scheme()` constructor.

- **User theme registry (`ThemeRegistry`)** — new `ThemeRegistry` struct in `src/ui/theme_loader.rs` discovers and loads user-defined `.toml` theme files from the platform config directory (`~/.config/podcast-tui/themes/` on Linux, `%APPDATA%\podcast-tui\themes\` on Windows, `~/Library/Application Support/podcast-tui/themes/` on macOS) alongside the four bundled themes. The `:theme` command and its tab-completion now reflect all registered themes (bundled + user). Invalid theme files log a warning to stderr but do not prevent startup. Closes [#103](https://github.com/lqdev/podcast-tui/issues/103).
- **Audio module scaffold (Phase 1)** — new `src/audio/mod.rs` with `PlaybackBackend` trait, `AudioError`, `AudioCommand` enum, and `PlaybackStatus` struct; foundational audio infrastructure for future playback integration. Closes [#133](https://github.com/lqdev/podcast-tui/issues/133), part of [#132](https://github.com/lqdev/podcast-tui/issues/132).
- **Keybinding conflict detection** — `KeyHandler::validate()` detects duplicate chord assignments and unbound critical actions (Quit, ShowHelp, HideMinibuffer). Conflicts are logged as warnings to stderr at startup; a missing Quit binding aborts startup with a clear error. Closes [#99](https://github.com/lqdev/podcast-tui/issues/99).
- **Keybinding presets and auto-generated help** — three built-in presets (`default`, `vim`, `emacs`) selectable via `"keybindings": { "preset": "vim" }` in `config.json`. Vim preset adds `hjkl` navigation and removes `h` from show-help. Emacs preset uses `C-p`/`C-n` and removes `j`/`k` aliases. User-level overrides in `global` apply on top of any preset. The `*Help: Keybindings*` buffer (F1, `?`, or `:help`) is now auto-generated from active bindings, always reflecting the current preset and any custom overrides. Closes [#100](https://github.com/lqdev/podcast-tui/issues/100).

**TOML Theme File Format and Parser — February 2026**
- **User-defined themes via `.toml` files** — new `src/ui/theme_loader` module parses TOML theme files into the existing `ColorScheme`/`Theme` structs; groundwork for loading themes from the filesystem in [#103](https://github.com/lqdev/podcast-tui/issues/103)
- **`load_theme_file(path: &Path) -> Result<Theme, ThemeError>`** — public API that reads a TOML file, resolves the base theme via `extends`, applies color overrides, and returns a `Theme` ready for use
- **`parse_color(s: &str) -> Result<Color, ThemeError>`** — public color parser supporting four formats:
  - Hex: `"#rrggbb"` (e.g. `"#ff79c6"` → `Color::Rgb(255, 121, 198)`)
  - RGB function: `"rgb(r, g, b)"` (e.g. `"rgb(255, 121, 198)"`)
  - Indexed: `"color(n)"` for 256-color terminals (e.g. `"color(141)"`)
  - Named: `"Red"`, `"Blue"`, `"DarkGray"`, `"reset"`, etc. (case-insensitive, all Crossterm names)
- **Theme inheritance via `extends`** — a theme file can specify `extends = "dark"` (or `light`, `high-contrast`, `solarized`) to inherit all colors from a bundled theme and override only the fields it specifies; missing `extends` defaults to the dark theme
- **Graceful invalid color handling** — unknown color strings produce a descriptive `ThemeError::InvalidColor` with the bad value and a hint about valid formats; invalid TOML produces `ThemeError::TomlParse`
- **Theme file format** — `[metadata]` section (name, optional author/description/extends) + `[colors]` section (all 21 `ColorScheme` fields as optional strings)
- Dependency added: `toml = "0.8"` (serde-backed TOML parser)
- Tests added: 27 unit tests covering all color formats, case insensitivity, inheritance from all 4 bundled themes, partial override, named colors, mixed formats, invalid colors, and missing sections. Closes [#102](https://github.com/lqdev/podcast-tui/issues/102).

**Load Keybindings from Config at Runtime — February 2026**
- **Keybindings are now loaded from `config.json` at startup** — the `keybindings.global` section in your config overrides the default hardcoded bindings at launch
- **`KeyHandler::from_config(config: &KeybindingConfig) -> Self`** — new constructor that starts from defaults and applies any non-empty override lists from config; replaces the previous `KeyHandler::new()` call in the UI startup path
- **Empty field = keep default** — a `Vec<String>` field left as `[]` in config preserves the built-in default for that action; only non-empty fields are applied as overrides
- **Graceful invalid notation handling** — unrecognised key notation strings in a non-empty override list are silently skipped; if at least one notation is valid its chord(s) replace the defaults, but if all notations are invalid the action keeps its defaults intact (the list is treated as a no-op)
- **`KeyHandler::lookup(&chord) -> Option<&UIAction>`** — new read-only accessor for diagnostics and testing
- Config format: edit `keybindings.global` in `~/.config/podcast-tui/config.json`; use Helix-style notation (`"C-q"`, `"S-Tab"`, `"F1"`) — see `GETTING_STARTED.md` for reference
- Tests added: 5 unit tests covering default equivalence, override removes old bindings, empty vec preserves defaults, invalid notation skips gracefully, multiple chords all bound. Closes [#98](https://github.com/lqdev/podcast-tui/issues/98).

**Expand KeybindingConfig to Cover All Bindings — February 2026**
- **`KeybindingConfig` now covers all 60+ bindable actions** — replaces the previous 17-field flat struct (which was unused) with a structured, context-organized schema
- **`GlobalKeys` struct** — all global actions represented as `Vec<String>` (multiple keys per action, e.g., `["Up", "k", "C-p"]` for move-up), organized into sections: Navigation, Buffer navigation, Application control, Interaction, Podcast management, Episode actions, Playlist, OPML, Sync, Tab navigation
- **Buffer-specific override sections** — optional per-context structs (`PodcastListKeys`, `EpisodeListKeys`, `PlaylistKeys`, `DownloadKeys`, `SyncKeys`) allow per-buffer keybinding overrides without affecting other contexts; `None` by default (use global bindings)
- **`impl Default for GlobalKeys`** — defaults mirror the primary hardcoded bindings in `keybindings.rs`, including vim aliases (`j`/`k`/`g`/`S-G`), Emacs aliases (`C-n`/`C-p`), function-key shortcuts (`F1`–`F10`), and terminal-compatibility aliases (`BackTab`/`S-BackTab` for Shift+Tab, `S-?`/`S-:` for shift-variant punctuation, `S-X` for shift-variant delete)
- **Partial config support** — `#[serde(default)]` at struct level means a user can specify only the fields they want to override (e.g., `{"global": {"quit": ["C-q"]}}`) and all other bindings fill in from the defaults
- **Key notation format** uses Helix-style strings (`"C-x"`, `"S-Tab"`, `"F1"`, etc.) compatible with the `parse_key_notation()` function added in #96
- Config fields: `keybindings.global.*`, `keybindings.episode_list`, `keybindings.podcast_list`, `keybindings.playlist`, `keybindings.downloads`, `keybindings.sync`
- Tests added: 6 unit tests covering defaults coverage, defaults-match-keybindings, roundtrip serialization, partial JSON deserialization, empty config defaults, and buffer section partial override. Closes [#97](https://github.com/lqdev/podcast-tui/issues/97).

### Changed

**Fix Hardcoded Color Violations — February 2026**
- **Active buffer indicator** in the buffer list (`buffer_list.rs`) now uses `theme.active_indicator_style()` instead of hardcoded `Color::Yellow`
- **Error state** in the podcast list (`podcast_list.rs`) — both the border and text — now use `theme.error_style()` instead of hardcoded `Color::Red`
- **New `active_indicator` color field** added to `ColorScheme`; all 4 bundled themes (dark, light, high-contrast, solarized) provide an appropriate yellow/gold value
- **New `active_indicator_style()` method** on `Theme` for highlighting active/current items in lists
- All `ratatui::style::Color` literals used by UI buffer rendering (e.g., `src/ui/buffers/`) now live in `src/ui/themes.rs` — no hardcoded colors remain in buffer rendering code. Closes [#101](https://github.com/lqdev/podcast-tui/issues/101).

**Standardize Context-Dependent Key Semantics — February 2026**
- **`d` now consistently means "delete"** across all buffers; pressing `d` in the sync buffer no longer triggers a dry-run preview — it shows an informational message instead
- **`D` (Shift-D) triggers dry-run preview** in the sync buffer (previously `d`); `D` already opened episode downloads in episode contexts, making this a clean contextual override with no global conflicts
- Help text updated: `d → D (Shift-D)` in both the Device Sync and Keybinding Reference sections
- Tests updated: renamed dry-run test to cover `DownloadEpisode` action; added test confirming `DeletePodcast` in sync buffer shows a "nothing to delete" message. Closes [#95](https://github.com/lqdev/podcast-tui/issues/95).

### Added

**Key Notation Parser (String ↔ KeyChord) — February 2026**
- **Foundation for configurable keybindings**: new `key_parser` module (`src/ui/key_parser.rs`) converts human-readable key notation strings to `KeyChord` structs and back
  - Supports modifier prefixes: `C-` (Ctrl), `S-` (Shift), `A-`/`M-` (Alt), and combinations like `C-S-x`
  - Named keys: `Enter`, `Tab`, `Esc`, `Backspace`, `Delete`, `Space`/`SPC`, arrow keys, `Home`, `End`, `PgUp`/`PgDn`
  - Function keys `F1`–`F12` with optional modifier prefix (e.g., `C-F3`)
  - Single character literals (e.g., `q`, `?`, `3`)
  - `key_to_notation()` serializes a `KeyChord` back to its canonical string (full round-trip support)
  - Descriptive `KeyParseError` variants for empty input, unknown key names, and missing keys after modifier
  - Tests added: 31 unit tests covering all key types, modifiers, combinations, round-trips, and error cases. Closes [#96](https://github.com/lqdev/podcast-tui/issues/96).

- Added vim-style navigation keys `j`/`k` (down/up), `g` (top), `G` (bottom) as aliases for arrow keys, Home, and End in all list buffers. Also added `C-n`/`C-p` as global Emacs-style navigation aliases. Closes [#93](https://github.com/lqdev/podcast-tui/issues/93).

- Implemented `m` (mark played) and `u` (mark unplayed) keybindings for episodes. Pressing `m` on a selected episode marks it as played (persists to storage), `u` marks it as unplayed. Works in the episode list, What's New, and playlist detail buffers. Visual status updates immediately; storage is written asynchronously. Closes [#94](https://github.com/lqdev/podcast-tui/issues/94).
  - Keybindings: `m` → Mark episode as played, `u` → Mark episode as unplayed
  - Help text updated with `m`/`u` in episode management sections
  - Tests added: 6 unit tests (episode_list buffer), 2 keybinding unit tests, 3 integration tests (storage roundtrip)

- **Episode Favorites/Starring — February 2026**: Press `*` on any episode to toggle its favorite status; favorited episodes show a `★` prefix in all list views. Favorites persist across app restarts. Closes [#106](https://github.com/lqdev/podcast-tui/issues/106).
  - **`★` indicator**: favorited episodes display `★ <title>` in the episode list and What's New buffers
  - **`:filter-status favorited`** — filter episode lists to show only starred episodes; combine with other filters using AND logic
  - **`toggle_favorite` config field** — keybinding is user-configurable via `keybindings.global.toggle_favorite` in `config.json` (default: `["*"]`)
  - **`#[serde(default)]` on `Episode.favorited`** — existing episode JSON files without the field deserialize with `favorited: false` (fully backward compatible)
  - Tests added: 10 unit tests (4 model tests, 6 filter tests)

**Episode Sort Options — February 2026**: Sort episodes by date, title, duration, or download status with ascending/descending toggle. Closes [#108](https://github.com/lqdev/podcast-tui/issues/108).
  - **`o`** — cycle sort field: Date → Title → Duration → Status → Date (wraps)
  - **`O` (Shift-O)** — toggle sort direction: ascending (↑) ↔ descending (↓)
  - **Sort indicator in buffer title** — always visible, e.g. `Episodes: My Podcast [↓ Date]` or `Episodes: My Podcast [played | ↑ Title]` when a filter is also active
  - **`:sort <field>`** minibuffer command — set sort field by name: `date`, `title`, `duration`, `downloaded`
  - **`:sort-asc`** / **`:sort-desc`** — set sort direction via command
  - **Default sort preserved** — Date Descending (newest first) matches legacy behaviour; no change to existing episode order on upgrade
  - **Config-overridable keybindings** — `keybindings.global.cycle_sort_field` and `keybindings.global.toggle_sort_direction` in `config.json`
  - Tests added: 11 unit tests covering all 4 sort fields (asc + desc), field cycle, direction toggle, unknown field error, sort indicator strings, and sort-persists-through-filter-change

**Podcast Tags/Categories — February 2026**: Tag and filter your podcast subscriptions using freeform labels. Closes [#107](https://github.com/lqdev/podcast-tui/issues/107).
  - **`:tag <name>`** — add a tag to the selected podcast (tags are normalized to lowercase and trimmed; duplicates silently ignored)
  - **`:untag <name>`** — remove a tag from the selected podcast
  - **`:tags`** — list all unique tags used across all podcast subscriptions
  - **`:filter-tag <tag>`** — narrow the podcast list to show only podcasts with a given tag; combine with `:search` for AND filtering
  - **`[tag]` badges** rendered in the podcast list item for each tagged podcast
  - Tags persist in `podcast.json` via `tags: Vec<String>`; existing podcast files without the field deserialize with `tags: []` (fully backward compatible via `#[serde(default)]`)
  - Optimistic UI update pattern: tag changes apply in-memory immediately, then persist asynchronously; reverts on failure
  - Tests added: 19 unit tests (7 model tests, 4 filter tests, 8 podcast-list buffer tests)

### Fixed

- Fixed help text to remove ghost keybindings that were listed but not bound (`m`/`u` mark played/unplayed in episode list, `C-n`/`C-p` navigation in What's New buffer, and inaccurate Emacs-style entries in the keybindings reference). Closes [#92](https://github.com/lqdev/podcast-tui/issues/92).

---

## [1.9.0] - 2026-02-20

### Added

**Sync Buffer Phase 2: Saved Targets, Directory Picker, History Persistence — February 2026**
- **Saved Targets**: Up to 5 recently-used sync destinations are remembered and shown in the overview, ranked by use count
- **Ranger-Style Directory Picker**: Press `p` from the sync buffer overview to browse the filesystem and select a sync target directory
  - Platform-aware quick access: drive letters (Windows), `/Volumes` + `~/Music` (macOS), `/media` + `/mnt` (Linux)
  - `→` / `MoveRight` to enter a directory; `←` / `MoveLeft` to go up; `Enter` on a regular directory to select it as the target
  - `Esc` cancels picker and returns to overview
- **History Persistence**: Sync results are saved across sessions in `{data_dir}/sync_history.json` (up to 10 entries with file counts)
- **Target Persistence**: Saved targets and the active target are stored in `{data_dir}/sync_targets.json`
- **Overview Status Panel**: Shows active target path and last sync summary (timestamp, mode, files copied/deleted)
  - Falls back to persisted history when in-memory sync hasn't run yet
- Keybindings: `p` → open directory picker, `Enter` → activate selected target (overview), `→`/`←` → navigate picker
- Tests added: 12 unit tests

**Sync Buffer Phase 3: Dry-Run Preview, Progress View, Config Options — February 2026**
- **Dry-Run Preview Mode**: After pressing `d`, a tabbed preview shows exactly what would happen before committing
  - **To Copy** tab: files to be copied with individual and total sizes
  - **To Delete** tab: orphan files that would be removed from the device
  - **Skipped** tab: files already identical on the device
  - **Errors** tab: any errors encountered during the scan
  - `[` / `]` key bindings to cycle between tabs
  - Press `Enter` or `s` from the preview to confirm and start the real sync
  - Press `Esc` to cancel and return to Overview
  - Pressing `d` with an active saved target now triggers the dry-run immediately (no path prompt)
- **Live Progress View**: Real syncs now display a progress view with:
  - Byte-based progress bar showing `copied bytes / total bytes`
  - Currently-copying filename updated in real time
  - Running counters: Copied / Deleted / Skipped / Errors
  - Elapsed time (m:ss or h:mm:ss)
  - Auto-transitions back to Overview when the sync completes
- **`SyncProgressEvent` enum**: `sync_to_device()` now accepts an optional `progress_tx` channel for progress streaming. All existing callers pass `None` and are unaffected.
- **`file_sizes` in `SyncReport`**: File sizes are now stored in the report for display in the dry-run preview
- **New config fields** (both `false` by default for backward compatibility):
  - `sync_preview_before_sync`: if `true`, pressing `s` always shows the dry-run preview first
  - `sync_filter_removable_only`: if `true`, directory picker only shows removable/external drives
- **`AppEvent::DeviceSyncProgress`**: New app event for routing progress updates from the async sync task to the sync buffer
- **`[`/`]` keybindings**: Added globally as `PreviousTab` / `NextTab` UIActions for use in preview mode
- Tests added: 16 new unit tests (tab cycling, progress events, format_bytes, config backward compat)

### Fixed

**Sync Buffer Foundation — February 2026**
- **`PromptInput` action no longer silently dropped**: Any buffer that returns `UIAction::PromptInput(prompt)` from `handle_action()` now correctly opens the minibuffer input prompt (previously lost in catch-all `_ => {}`)
- **`s` key now triggers sync in sync buffer**: `s` is bound globally to `UIAction::SyncToDevice`; the sync buffer intercepts it and prompts for a device path
- **`d` key now triggers dry-run in sync buffer**: `d` (previously `DeletePodcast`) is intercepted when the sync buffer is active and opens a dry-run device path prompt
- **`r` key now works in sync buffer**: `r` (previously `RefreshPodcast`) is intercepted when the sync buffer is active for a clean re-render
- **F8 shortcut added**: Press `F8` from anywhere to switch to the sync buffer (consistent with F2=podcasts, F4=downloads, F7=playlists)
  - Keybinding: `F8` → sync buffer
  - Keybinding: `s` → sync to device (sync buffer only)
  - Keybinding: `d` → dry-run preview (sync buffer only)
  - Tests added: 5 unit tests

**Sync buffer inaccessible at startup — February 2026**
- **F8 and `:buffer sync` now work**: the sync buffer was missing from the `initialize()` code path that runs at startup, making it unreachable via F8 or `:buffer sync` ([#86](https://github.com/lqdev/podcast-tui/issues/86))

### Changed

- **Zero clippy warnings** ([#85](https://github.com/lqdev/podcast-tui/pull/85)): Fixed all 42 clippy warnings (`unnecessary_map_or`, `redundant_closure`, `redundant_pattern_matching`, missing `Default` impls, `large_enum_variant`); `cargo clippy -- -D warnings` now passes clean

---

## [1.8.0] - 2026-02-19

### Added

- **Enter key opens episode details in playlist** ([#58](https://github.com/lqdev/podcast-tui/pull/58)): Press `Enter` on a playlist entry to open the Episode Detail buffer (closes [#56](https://github.com/lqdev/podcast-tui/issues/56))
- **Add-to-playlist from Episode Detail and What's New** ([#60](https://github.com/lqdev/podcast-tui/pull/60)): The `p` keybinding now works in Episode Detail and What's New buffers to add an episode to a playlist (closes [#55](https://github.com/lqdev/podcast-tui/issues/55))
- **MockStorage test infrastructure** ([#70](https://github.com/lqdev/podcast-tui/pull/70)): `MockStorage` auto-generated via `mockall::automock` on the `Storage` trait; includes failure-tracking test for `cleanup_old_downloads_hours`

### Changed

- **Documentation overhaul** ([#61](https://github.com/lqdev/podcast-tui/pull/61)): Comprehensive rewrite of README, ARCHITECTURE, GETTING_STARTED, KEYBINDINGS, and all feature docs
- **Friendly error messages** ([#62](https://github.com/lqdev/podcast-tui/pull/62)): `StorageError` variants now display human-readable copy (e.g., "Could not find podcast.") instead of internal UUIDs and file paths; `technical_details()` method preserves internals for logging
- **F3 remapped to Search** ([#65](https://github.com/lqdev/podcast-tui/pull/65)): `F3` now triggers `Search`, consistent with VS Code and Vim conventions. Previous `SwitchBuffer("*Help*")` binding was silently a no-op. Help remains on `F1`, `h`, and `?`

### Fixed

- **Selection feedback severity** ([#66](https://github.com/lqdev/podcast-tui/pull/66)): "No X selected" messages across playlist buffers changed from `show_error` to `show_message`
- **`PodcastId::as_str()` memory leak** ([#67](https://github.com/lqdev/podcast-tui/pull/67)): Removed unsafe implementation that leaked heap-allocated strings; replaced with safe alternative
- **Error context in download cleanup** ([#68](https://github.com/lqdev/podcast-tui/pull/68)): `cleanup_old_downloads_hours` failure paths now include podcast-level context in error messages

---

## [1.7.0] - 2026-02-18

### Added

**User Playlists & Today Playlist — February 2026**
- **User Playlists**: Create, delete, and manage custom playlists with copied audio files for device compatibility
- **Auto-Generated `Today` Playlist**: Rolling 24-hour playlist using episode `published` date, with refresh policies (`daily`, `on_launch`, `manual`)
- **Playlist Storage**: New `playlists/` data directory with per-playlist metadata (`playlist.json`) and `audio/` files (`001-Title.ext` ordering)
- **Playlist UI**:
  - New playlists buffer (F7)
  - Playlist detail and picker buffers
  - Episode action `p` to add to playlist
  - Playlist commands: `:playlists`, `:playlist-create`, `:playlist-delete`, `:playlist-refresh`, `:playlist-sync`
- **Sync Integration**:
  - `:sync` now syncs both downloads and playlists by default
  - Device output layout now uses sibling directories:
    - `<sync_device_path>/Podcasts/...`
    - `<sync_device_path>/Playlists/<playlist>/...`
  - New config option: `downloads.sync_include_playlists` (default: `true`)
- **Configuration**: Added `playlist` config section for refresh policy and playlist download behavior
- **Hard sync (`--hard`) mode**: Wipes managed `Podcasts/` and `Playlists/` directories on the device before a fresh copy, while preserving unmanaged files

### Fixed

- **Device sync orphan scope**: Orphan file deletion now scoped to managed `Podcasts/` and `Playlists/` roots only — regular sync no longer risks deleting unrelated audio files on the device
- **Safe buffer downcasting**: Replaced unsafe raw pointer casts with `Any` trait for correct buffer type dispatch
- **Today playlist filename normalization**: Retained episodes are re-copied with correct download-stem filenames on refresh

---

## [1.6.0] - 2025-11-XX

### Added

**Search & Filter — November 2025**
- **Text Search**: `/` key opens search bar; case-insensitive match against episode title and description
- **Status Filter**: `:filter-status <status>` — filter episodes by `new`, `downloaded`, `played`, `downloading`, or `failed`
- **Date Range Filter**: `:filter-date <range>` — filter by `today`, `7d` (last 7 days), `2w` (2 weeks), `1m` (1 month)
- **Clear Filters**: `:clear-filters` removes all active filters and restores full episode list
- **Filter Logic**: All active filters are combined with AND semantics (episode must match all)
- **Tab Completion**: All filter commands support tab completion in the minibuffer
- **EpisodeFilter Model**: New `src/ui/filters.rs` with `EpisodeFilter`, `EpisodeStatus`, `DateRange` types
- **Duration Filter**: Architecture support present; deferred pending RSS duration data (Design Decision #13)

**Winget Publishing — November 2025**
- Added winget manifest files for Windows Package Manager publication
- `winget install lqdev.PodcastTUI` now available
- Added `docs/WINGET_PUBLISHING.md` with publishing workflow documentation

---

## [1.5.0-mvp] - 2025-10-XX

### Added

**Download Cleanup — Auto-Cleanup on Startup and Manual Command**
- **Auto-cleanup on Startup**: Automatically delete downloaded episodes older than the configured `cleanup_after_days` threshold when the app launches
  - Wires up the previously-dead `cleanup_after_days` config field (default: 30 days)
  - Silent when nothing to clean; shows count when episodes are removed
  - Disabled when `cleanup_after_days` is `null` or `0` in config
- **Manual `:clean-older-than <duration>` Command**: Delete downloads older than a specified duration
  - Supports flexible duration syntax: `12h` (hours), `7d` (days), `2w` (weeks), `1m` (months)
  - Bare numbers default to days (e.g., `30` = 30 days)
  - Confirmation prompt before deletion to prevent accidental data loss
  - Alias `:cleanup` for convenience
  - Tab-completion support for both command names
- **Duration Parser**: New `parse_cleanup_duration()` and `format_cleanup_duration()` utility functions
  - Case-insensitive, validates range (>= 1h, <= 365d)
  - Singular/plural formatted output (e.g., "1 week", "2 months")
  - 11 comprehensive unit tests
- **Download Manager**: New `cleanup_old_downloads()` and `cleanup_old_downloads_hours()` methods
  - Uses file modification time to determine download age (no schema migration needed)
  - Follows the same pattern as `delete_all_downloads()` for consistency
  - Cleans up empty directories after deletion
- **Help Buffer**: Added DOWNLOAD CLEANUP section with command reference
- **Documentation**: Updated `docs/KEYBINDINGS.md` with cleanup command reference

---

## [1.4.0-mvp] - 2025-10-XX

### Added

**Device Sync for MP3 Players**
- **Metadata-Based Device Sync**: Sync downloaded episodes to external MP3 players or USB devices
  - Compare files using metadata only (filename + file size) for fast, reliable sync
  - Runtime device path override - specify sync target when initiating sync
  - Preserves podcast folder structure on device for easy navigation
  - Dry-run mode for safe preview of sync changes before execution
  - Orphan file deletion - removes episodes on device that are no longer on PC
  - Atomic operations with comprehensive error handling and reporting
  - New Sync buffer with visual history of last 10 sync operations
  - Shows sync status, file counts, timestamps, and success/failure indicators
  - Commands: `sync <path>` for full sync, `sync-dry-run <path>` for preview
  - Buffer aliases: `sync`, `device-sync` for quick navigation
  - Configuration options in `config.json`:
    - `sync_device_path`: Optional default device path
    - `sync_delete_orphans`: Auto-delete orphaned files (default: true)
    - `sync_preserve_structure`: Keep folder hierarchy (default: true)
    - `sync_dry_run`: Default mode for safety (default: false)
  - 7 comprehensive unit tests covering all sync scenarios
  - Built for MP3 player compatibility with existing ID3 metadata features

### Added

**Application Icon**
- **Custom Application Icon**: Added icon for easy identification in system UI
  - Created SVG icon combining cassette tape and RSS feed symbols
  - Generated PNG versions in multiple sizes (16x16, 32x32, 48x48, 64x64, 128x128, 256x256)
  - Created Windows ICO file with multi-resolution support
  - Implemented Windows icon embedding via `build.rs` using `winres` crate
  - Icon automatically appears in Windows taskbar, Task Manager, and file explorer
  - Created Linux desktop entry file for application launcher integration
  - Added `install-icon-linux.sh` script for Linux icon installation
  - Icons installed to `~/.local/share/icons/hicolor/` following XDG standards
  - Desktop entry installed to `~/.local/share/applications/`
  - Added `regenerate-icons.sh` utility script for rebuilding icons from SVG
  - Updated build scripts to include icon assets in release packages
  - Comprehensive documentation in `assets/README.md`
  - Cross-platform support: Windows (embedded) and Linux (system icons)

### Fixed

**Omny.fm Podcast Downloads - November 2025**
- **Corrupted Download Files**: Fixed episodes from Omny.fm hosted podcasts not downloading correctly
  - **Root Cause**: Servers were returning HTTP 200 OK with HTML error pages instead of audio files
  - Added HTTP status validation in `download_file()` method using `error_for_status()`
  - **Added Content-Type validation** to reject HTML responses with clear error messages
  - Validates `Content-Type` header before downloading (accepts audio/*, video/*, octet-stream)
  - Rejects downloads when Content-Type contains "html" even with 200 OK status
  - Added HTTP status validation in `download_artwork()` method
  - Standardized User-Agent string to match feed parser for consistent server behavior
  - Fixes corruption issues with Desert Oracle, Better Offline, and other Omny.fm podcasts
  - Ensures podcast cover art downloads correctly
  - No impact on existing functionality - all 97 unit tests pass
  - Provides clear error: "Server returned HTML instead of audio file"

**UI Thread Blocking - October 10, 2025**
- **Responsive UI During Background Operations**: Fixed UI thread blocking during podcast refresh and downloads
  - Moved all buffer refresh operations to background tasks using `tokio::spawn`
  - Created `BufferRefreshType` enum to categorize different refresh operations
  - Implemented `BufferDataRefreshed` app event to send pre-loaded data back to UI thread
  - Added `set_downloads()`, `set_episodes()`, and `set_podcasts()` methods for non-blocking buffer updates
  - Refactored podcast list, downloads, What's New, and episode buffer refreshes to use background loading
  - Fixed F5 refresh action to use background refresh system instead of blocking `.await` calls
  - UI now remains fully responsive during podcast refreshes, downloads, and data loading operations
  - Users can scroll, navigate, and switch buffers while background operations are running
  - Eliminates UI freezing and weird character artifacts during intensive operations

**Episode Description Rendering - October 2025**
- **HTML Content in Descriptions**: Fixed rendering of episode descriptions containing HTML/CDATA
  - Added `utils::text` module with HTML stripping functionality
  - Implemented `strip_html()` function to remove HTML tags from RSS feed content
  - Added HTML entity decoding for common entities (&amp;, &lt;, &gt;, etc.)
  - Added smart whitespace cleanup to handle excessive newlines and spaces
  - Applied sanitization to both episode and podcast descriptions in feed parser
  - Resolves issue with feeds like Audioboom that include raw HTML in descriptions
  - Clean text feeds (like Libsyn) remain unchanged
  - Comprehensive test coverage with 10 unit tests

**GitHub Actions Release Build - October 2025**
- **Release Build Workflow**: Fixed failing GitHub Actions release build workflow
  - Fixed Zig installation PATH issue where `zig` binary wasn't accessible after pip install
  - Created symlink from `~/.local/bin/zig` to the ziglang package binary
  - Added missing `aarch64-unknown-linux-gnu` Rust target to installation script
  - Improved PATH handling to ensure zig is available for cargo-zigbuild
  - Updated `scripts/install-build-deps.sh` with proper GitHub Actions PATH integration
- **GitHub Release Creation**: Fixed 403 error when creating GitHub releases
  - Added `permissions: contents: write` to `create-release` job in `.github/workflows/release.yml`
  - GitHub Actions now requires explicit permissions for GITHUB_TOKEN to create releases
  - Resolves "⚠️ GitHub release failed with status: 403" error

### Added

**Code Quality & Documentation Improvements - October 2025**
- **Constants Module**: Centralized configuration defaults in `src/constants.rs`
  - 8 organized categories (network, filesystem, downloads, ui, storage, feed, audio, opml)
  - 240 lines with comprehensive documentation
  - Validated in unit tests
  - Eliminated all magic numbers from codebase
- **Architecture Documentation**: Created comprehensive `docs/ARCHITECTURE.md` (500+ lines)
  - Core architectural principles and design patterns
  - Module structure and dependencies
  - Storage abstraction design
  - UI component patterns
  - Data flow diagrams
- **Testing Documentation**: Created comprehensive `docs/TESTING.md` (450+ lines)
  - Testing philosophy and goals
  - Test categories and organization
  - Component-specific testing strategies
  - Test implementation roadmap
  - Quality guidelines and best practices

### Changed

**Documentation Organization - October 2025**
- Reorganized root directory from 27 files to ~12 essential files
- Archived 19 historical documents to `docs/archive/` structure
- Created `docs/archive/cleanup/` for cleanup progress tracking documents
- Enhanced `CONTRIBUTING.md` with architecture and testing references
- Updated `README.md` with clear documentation hierarchy
- Updated `IMPLEMENTATION_PLAN.md` with accurate sprint status (3/8 complete = 37.5%)
- Added comprehensive cross-references between documentation files

### Removed

**Documentation Cleanup - October 2025**
- Removed 5 redundant documentation files:
  - `BUILD_COMMANDS.md` (consolidated into `docs/BUILD_SYSTEM.md`)
  - `BUILD_SYSTEM_FINAL.md` (consolidated into `docs/BUILD_SYSTEM.md`)
  - `GIT_COMMIT_INFO.md` (outdated)
  - `QUICKSTART.md` (merged into `GETTING_STARTED.md`)
  - `docs/BUILD_SYSTEM_SUMMARY.md` (consolidated into main build docs)

### Fixed

**Code Quality - October 2025**
- Fixed 8 clippy warnings (unused imports, visibility issues, dead code markers)
- Removed all magic numbers by introducing constants module
- Improved code maintainability with single source of truth for defaults
- Enhanced error handling with proper field prefixes for intentional unused values

### Added

**OPML Import/Export Support - ✅ COMPLETE**
- **OPML Import**: Import podcast subscriptions from OPML files or URLs
  - Support for local files and remote URLs
  - Non-destructive import (skips duplicates)
  - Sequential processing with progress callbacks
  - Detailed logging with timestamped import reports
  - Real-world OPML compatibility (handles malformed XML)
  - Robust XML sanitization for unescaped entities
  - Flexible `@text`/`@title` attribute handling
- **OPML Export**: Export current subscriptions to OPML 2.0 format
  - Configurable export directory
  - Timestamped filenames for version tracking
  - Atomic writes for data safety
  - Full OPML 2.0 compliance with both `@text` and `@title` attributes
- **UI Integration**: Seamless OPML workflows
  - Keybindings: `Shift+A` (import), `Shift+E` (export)
  - Commands: `:import-opml`, `:export-opml`
  - Real-time progress feedback
  - Detailed import/export summaries
- **Error Handling**: Comprehensive error reporting
  - Validation errors with clear messages
  - Network error handling with retries
  - Per-feed error tracking during import
  - Detailed log files for troubleshooting

### Fixed
- **OPML Real-World Compatibility**: Fixed parsing of OPML files from popular services
  - Handle OPML files missing required `@text` attribute (fall back to `@title`)
  - Sanitize unescaped ampersands and special characters in XML attributes
  - Support OPML files that violate strict XML/OPML specifications
- **Minibuffer Context Preservation**: Fixed race condition in minibuffer input handling
  - Prompt context now captured before submit() clears state
  - OPML import from URLs no longer misidentified as podcast subscription

**Sprint 3: Downloads & Episode Management (Week 4) - ✅ COMPLETE**
- **Download System**: Full parallel download implementation with progress tracking
  - Concurrent download manager supporting 2-3 parallel downloads (configurable)
  - Progress tracking with byte-level granularity
  - Resume capability for interrupted downloads
  - Automatic cleanup of downloaded episodes (configurable age-based)
  - Bulk delete functionality for podcast downloads
- **Episode Management**: Complete episode browsing and organization
  - Episode list buffer with status indicators (new/downloaded/played)
  - Episode metadata display with comprehensive information
  - Download status integration throughout UI
  - File organization by podcast with sanitized filenames
- **File Management**: Robust file handling
  - Configurable download directory with expansion support
  - Safe file naming with special character handling
  - Atomic file operations for reliability
  - Year-based organization option

**Sprint 2: RSS & Podcast Functionality (Week 3) - ✅ COMPLETE**
- **RSS Feed Parsing**: Full RSS/Atom feed support with feed-rs integration
  - Multi-strategy audio URL extraction (6 different strategies)
  - Comprehensive feed validation and error handling
  - Support for various feed formats and quirks
  - Metadata extraction (title, description, author, artwork)
- **Subscription Management**: Complete podcast subscription system
  - Subscribe/unsubscribe functionality with duplicate prevention
  - Podcast list with sorted display (by last updated)
  - Feed refresh with smart episode detection
  - Hard refresh option to update existing episodes
- **OPML Support**: Import/export podcast subscriptions
  - Non-destructive OPML import with duplicate detection
  - Import from local files or HTTP(S) URLs
  - Sequential processing with real-time progress updates
  - Detailed error logging for failed imports
  - Standard OPML 2.0 compliant export format
  - Configurable export directory with timestamped filenames
  - Batch subscription handling
  - Keyboard shortcuts: `Shift+A` (import), `Shift+E` (export)
  - Commands: `:import-opml` and `:export-opml`
- **Episode Detection**: Intelligent episode management
  - Deterministic episode IDs based on GUID for deduplication
  - Multi-strategy duplicate detection (GUID, URL, title+date)
  - Track number assignment for episodes
  - Episode status tracking (new/downloaded/played)

**Sprint 1: Core UI Framework (Week 2) - ✅ COMPLETE**
- **Core UI Framework**: Complete Emacs-style TUI framework implementation
  - Comprehensive UIAction system with 20+ action types for navigation and control
  - Full event handling system with crossterm integration and async support
  - Sophisticated keybinding system with prefix key support (C-x, C-h, C-c sequences)
  - Emacs-style navigation keys (C-n/C-p/C-f/C-b) with arrow key alternatives
- **Buffer Management System**: True Emacs-style buffer paradigm
  - Buffer trait for extensible content types with proper lifecycle management
  - BufferManager with buffer switching, next/previous navigation
  - Help buffer with scrollable keybinding documentation and custom content support
  - Placeholder podcast list and episode list buffers for upcoming RSS functionality
- **UI Components**: Professional-grade terminal UI components
  - **Minibuffer**: Full input handling, command history, cursor movement, message display
  - **Status Bar**: Real-time buffer name display, key sequence feedback, contextual help hints
  - **Theme System**: 4 complete themes (dark, light, high-contrast, solarized) with consistent styling
- **Main Application Loop**: Robust async application framework
  - Complete UIApp with 60fps event loop, efficient rendering, comprehensive action handling
  - Command execution system supporting M-x Emacs-style commands (quit, help, theme, buffer)
  - Dynamic theme switching, buffer switching, integrated help system
  - Proper async event handling with terminal cleanup and error recovery
- **Sprint 0 Foundation**: Complete Rust project structure and backend systems
  - **Storage Layer**: Comprehensive JSON-based storage with abstraction trait
  - **Data Models**: Rich domain models for Podcast, Episode, and configuration  
  - **Configuration System**: JSON-based configuration with sensible defaults
  - **Utilities**: Helper modules for file system, time formatting, and validation

### Changed
- Updated implementation plan to reflect completed Sprint 0 and Sprint 1 objectives
- Enhanced README with current MVP progress status showing Sprint 1 completion
- Improved git repository hygiene with proper .gitignore for Rust projects

### Fixed  
- Episode `is_played()` logic now correctly respects status vs historical play count
- Storage layer properly handles serialization errors without anyhow dependencies
- Git repository size issues by removing build artifacts and adding comprehensive .gitignore
- All compilation errors in UI framework with proper async/trait implementations
- Theme system now supports "default" theme alias for better backwards compatibility

## Sprint Progress

### Sprint 0: Project Setup (Week 1) - ✅ **COMPLETE**
**Completed Objectives:**
- ✅ Project structure with modern Rust tooling
- ✅ Storage abstraction layer with comprehensive JSON backend
- ✅ Rich data models with full test coverage
- ✅ Configuration management system
- ✅ Application architecture foundation
- ✅ Development environment and CI setup

**Key Achievements:**
- 19 passing unit tests covering core functionality
- Comprehensive error handling following Rust best practices  
- Clean separation of concerns (Storage trait abstraction)
- Atomic file operations for data consistency
- Rich domain models supporting MVP feature requirements
- Proper async/await implementation throughout

### Sprint 1: Core UI Framework (Week 2) - ✅ **COMPLETE**
**Completed Objectives:**
- ✅ Complete Emacs-style TUI framework with ratatui and crossterm
- ✅ Comprehensive event handling system with async support
- ✅ Sophisticated keybinding system with prefix keys (C-x, C-h, C-c)
- ✅ Buffer management system following Emacs paradigms
- ✅ Professional UI components (minibuffer, status bar, themes)
- ✅ Main application loop with 60fps rendering and proper cleanup
- ✅ Command execution system (M-x commands)
- ✅ Multi-theme support with dynamic switching

**Key Achievements:**
- **60 passing unit tests** covering all UI framework components
- Complete Emacs-style navigation (C-n, C-p, C-f, C-b) with alternatives
- Robust async event loop with proper terminal management
- Extensible buffer system ready for RSS/podcast content
- Professional theming system with accessibility considerations
- Comprehensive error handling and recovery throughout UI stack
- Full integration testing of UI workflows and interactions

**Next Up: Sprint 2 - RSS & Podcast Functionality**

## [1.3.1-mvp] - 2025-09-XX
See git log for changes in this release (application icon, cross-platform builds).

## [1.3.0-mvp] - 2025-09-XX
See git log for changes in this release.

## [1.2.0-mvp] - 2025-09-XX
See git log for changes in this release (background buffer refresh for responsive UI).

## [1.1.0-mvp] - 2025-09-XX
See git log for changes in this release.

## [1.0.0-mvp] - 2025-09-XX
Initial MVP release: Core UI framework, RSS subscription management, episode browsing, downloads, OPML import/export.

---

## Release Planning

### Version History
All tagged releases and their major features:
- **v1.9.0**: Sync buffer overhaul (Phase 2 + 3: saved targets, dry-run preview, live progress), sync buffer keybinding fixes, clippy clean pass
- **v1.8.0**: Playlist UX polish, docs overhaul, friendly error messages, F3→Search, MockStorage tests
- **v1.7.0**: User playlists, Today playlist, hard sync mode, device sync orphan scope fix
- **v1.6.0**: Search & filter, winget publishing
- **v1.5.0-mvp**: Download cleanup (auto + manual)
- **v1.4.0-mvp**: Device sync for MP3 players
- **v1.3.1-mvp**: Application icon (Windows embedded + Linux desktop entry)
- **v1.3.0-mvp**: Version fix, background buffer refresh
- **v1.2.0-mvp**: Background buffer loading for responsive UI
- **v1.1.0-mvp**: OPML import/export, code quality improvements
- **v1.0.0-mvp**: Initial MVP release

### Breaking Changes Policy

For 1.x releases:
- Configuration format changes will include migration tools
- Storage format changes will include automatic migration
- Major breaking changes will increment the major version number
- Deprecated features will be supported for at least one minor version
