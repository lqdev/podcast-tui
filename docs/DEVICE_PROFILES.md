# Device Profiles

Device profiles let Podcast TUI rewrite the filenames of episodes copied
to an MP3 player or USB drive **without** changing how those files are
stored on your computer. The local downloads directory keeps the rich,
metadata-tagged readable filenames; the device gets whatever the active
profile's template renders.

This is necessary for devices that ignore ID3 tags and display the bare
filename as the track title. The headline example is the **Innioasis Y1**,
which shows the literal filename and offers no metadata fallback.

> **Status:** runtime profile switching with `:set-device-profile` lands in
> v1.12.0. Persisting that switch back to `config.json` is tracked in
> [#223](https://github.com/lqdev/podcast-tui/issues/223).

## Quick start: Innioasis Y1

Add this to `~/.config/podcast-tui/config.json` (Linux) or
`%APPDATA%\podcast-tui\config.json` (Windows):

```json
{
  "device_profiles": [
    {
      "name": "Innioasis Y1",
      "match_path_contains": "INNIOASIS",
      "filename_template": "{podcast_short}/{track:03} - {title}.{ext}",
      "max_filename_length": 64,
      "ascii_only": true,
      "preserve_structure": false
    }
  ],
  "active_device_profile": "Innioasis Y1"
}
```

Restart the app, then open the Sync buffer (`F8`). The header shows
`Active device profile: Innioasis Y1`. A dry-run (`d`) previews the
renamed files; `s` syncs them.

## Schema

Each entry in `device_profiles` is a `DeviceProfile`:

| Field                 | Type    | Default | Description                                                                                                                          |
|-----------------------|---------|---------|--------------------------------------------------------------------------------------------------------------------------------------|
| `name`                | string  | —       | Human-readable identifier. Referenced by `active_device_profile`.                                                                    |
| `match_path_contains` | string? | `null`  | Substring matched against the sync target path. Currently informational; auto-selection is not yet wired to it.                      |
| `filename_template`   | string  | —       | Template used to render the per-file device-side path. See [Token reference](#token-reference). Empty templates are rejected at sync time. |
| `max_filename_length` | uint    | `128`   | Maximum **byte** length of the rendered filename (per path segment, excluding separators). The title segment is truncated to fit.    |
| `ascii_only`          | bool    | `false` | If `true`, transliterate or strip non-ASCII characters from the rendered name.                                                       |
| `preserve_structure`  | bool    | `true`  | If `true`, keep per-podcast subdirectories (matching the local layout). If `false`, flatten every file under the device's `Podcasts/` root. |

`active_device_profile` (top-level) selects which profile to apply. Set to
`null` (or omit) to use the default behaviour: copy files verbatim with
no template rewriting.

Profiles are pure config — they do not affect how files are stored
locally, only how they are written during `sync_to_device`. See
`DeviceProfile` in [`src/config.rs`](../src/config.rs) for the full
serde-typed definition.

## Token reference

The full reference also lives at the top of
[`src/download/device_template.rs`](../src/download/device_template.rs).

| Token                  | Substitution                                                          |
|------------------------|-----------------------------------------------------------------------|
| `{podcast}`            | Podcast title, sanitized                                              |
| `{podcast_short}`      | Podcast title, sanitized then truncated to 30 chars                   |
| `{title}`              | Episode title, sanitized                                              |
| `{track}`              | Episode number (no padding); empty string if missing                  |
| `{track:NN}`           | Episode number, zero-padded to N digits (`{track:03}` → `007`)        |
| `{episode_number}`     | Alias for `{track}`                                                   |
| `{episode_number:NN}`  | Alias for `{track:NN}`                                                |
| `{date}`               | Published date, default format `YYYY-MM-DD`                           |
| `{date:%fmt}`          | Published date with `chrono` strftime format (e.g. `{date:%Y-%m}`)    |
| `{ext}`                | File extension (e.g. `mp3`) without leading dot                       |

A literal `/` in the template creates a subdirectory on the device.
Slashes that appear *inside* a substituted value (e.g. a podcast title
containing `/`) are sanitized away — they never create unintended
subdirectories.

## Worked examples

### 1. Innioasis Y1 (no metadata, short filenames)

```json
{
  "name": "Innioasis Y1",
  "filename_template": "{podcast_short}/{track:03} - {title}.{ext}",
  "max_filename_length": 64,
  "ascii_only": true,
  "preserve_structure": false
}
```

Renders as: `My Tech Podcast/007 - Episode about Rust.mp3`

`{podcast_short}` keeps directory names readable on small displays.
`ascii_only: true` strips emoji and accented characters that the Y1's
font cannot render. `max_filename_length: 64` matches the Y1's display
limit.

### 2. Generic flat layout

```json
{
  "name": "Flat",
  "filename_template": "{podcast} - {title}.{ext}",
  "max_filename_length": 128,
  "ascii_only": false,
  "preserve_structure": false
}
```

Renders as: `My Tech Podcast - Episode about Rust.mp3`, all files in a
single device folder. Useful for devices that don't handle nested
directories well.

### 3. Date-organised by month

```json
{
  "name": "ByMonth",
  "filename_template": "{podcast}/{date:%Y-%m}/{title}.{ext}",
  "preserve_structure": false
}
```

Renders as: `My Tech Podcast/2026-05/Episode about Rust.mp3`. The
`{date:%Y-%m}` token uses chrono's strftime format — anything chrono
accepts works here.

## Switching profiles at runtime

Use the `:set-device-profile` minibuffer command:

```text
:set-device-profile Innioasis Y1
```

Tab-completes against `device_profiles[].name`. Pass an empty argument
(`:set-device-profile`) to clear the active profile. The Sync buffer
header updates immediately.

> **Limitation:** the change is in-memory only — it does not write back
> to `config.json` and does not survive a restart. Tracked in
> [#223](https://github.com/lqdev/podcast-tui/issues/223). To make a
> change permanent, edit `config.json` directly.

## Verifying before syncing

Set `downloads.sync_preview_before_sync` to `true` in `config.json`:

```json
{
  "downloads": {
    "sync_preview_before_sync": true
  }
}
```

Now `s` in the Sync buffer always shows the rename plan first; you have
to confirm before any files are copied. You can also trigger a one-off
preview with `d` (dry-run) or `:sync-dry-run [path]` regardless of this
setting.

## Troubleshooting

**Two episodes render to the same filename after templating.**
After applying the template, `max_filename_length`, and `ascii_only`,
collisions are disambiguated by appending the last 6 characters of the
episode UUID. You'll see filenames like
`007 - Episode about Rust-a1b2c3.mp3` in the second one.

**My non-ASCII titles look weird with `ascii_only: true`.**
The transliterator is `unicode_folding`-style: accented Latin characters
are folded to their base letter (`é` → `e`), and characters with no
ASCII equivalent (CJK, emoji) are stripped. If the result is empty, the
template will fall back to the episode UUID prefix so you never get a
zero-length filename.

**Filenames get cut off mid-word.**
`max_filename_length` truncates the title segment (not the podcast
folder, not the extension) to keep the rendered name within the byte
budget. If the cut is unacceptable, raise `max_filename_length` (your
device's actual limit may be higher than 64) or pick a template with
fewer leading tokens (`{title}.{ext}` rather than
`{podcast_short}/{track:03} - {title}.{ext}`).

**`set-device-profile` says "unknown profile".**
The argument is matched exactly against the `name` field, including
case and whitespace. Tab-completion will only offer names that exist.

**Files outside `Podcasts/` aren't being renamed.**
Templates only apply to files copied into the device's `Podcasts/`
subtree. Playlists in `Playlists/` and any other top-level files are
forwarded verbatim. `.m3u` playlist files are not yet rewritten to
point at the renamed device files (tracked as a known follow-up).

## See also

- [`docs/KEYBINDINGS.md`](KEYBINDINGS.md) — `:sync`, `:sync-dry-run`,
  `:set-device-profile` reference.
- [`docs/STORAGE_DESIGN.md`](STORAGE_DESIGN.md) — how the local
  downloads directory is laid out (which is what gets *copied from*).
- [`src/config.rs`](../src/config.rs) — `DeviceProfile` and
  `Config::active_device_profile()`.
- [`src/download/device_template.rs`](../src/download/device_template.rs)
  — template engine, sanitiser, disambiguator.
