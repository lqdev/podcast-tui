---
name: add-device-profile
description: Add a new device profile to podcast-tui's config.json so episodes sync to an MP3 player / DAP / USB drive with device-appropriate filenames and folder layout. Covers device research, field selection, and safe config editing.
---

# Skill: Add a Device Profile

## When to use
When a user wants podcast-tui to sync to a specific device (MP3 player, DAP,
USB drive) and asks to "create/add a profile" for it. Device profiles are pure
config — they rewrite device-side filenames during `sync_to_device` without
changing the local downloads layout. No code changes are required.

See `docs/DEVICE_PROFILES.md` for the user-facing reference and full token
list, and `DeviceProfile` in `src/config.rs` for the serde-typed schema.

## Step 1 — Research the device (the important part)

The JSON edit is trivial; classifying the device correctly is the real work.
Use web search / product reviews to answer four questions, because each maps
directly to a profile field:

| Question | Field it drives | Why it matters |
|---|---|---|
| **How does it connect to a PC?** | (workflow, not a field) | `sync_to_device` just needs a real filesystem path that exists and is writable (`device_path.exists()`/`is_dir()` in `src/download/manager.rs`) — it doesn't care about the transport. **USB Mass Storage** (e.g. Snowsky Echo Nano) mounts as a drive letter → sync directly. **MTP** (e.g. HiBy R4, most Android DAPs) exposes no drive letter on **Windows**, so there the practical option is to sync the microSD in a **card reader**. On Linux/macOS an MTP device *can* be mounted to a path (gvfs/jmtpfs), in which case that mount point works as the sync target. |
| **Does it read ID3 tags / render Unicode?** | `ascii_only` | Rich players (FiiO, HiBy, most Android) → `false`. Budget players that show the raw filename and can't render Unicode (e.g. Innioasis Y1) → `true`. |
| **Does it browse subfolders?** | `preserve_structure` | `true` keeps the managed `Podcasts/` root prefix on device paths; the per-podcast `Podcasts/<podcast>/…` grouping only happens if the `filename_template` starts with a `{podcast}/` segment. `false` flattens everything to the device root (separators in the rendered template become `_`) and **skips orphan deletion** — see DEVICE_PROFILES.md. |
| **How small is the screen / filename limit?** | `max_filename_length`, `filename_template` | Tiny displays or strict limits → shorter template and/or lower cap. Default cap is `128` bytes per path segment. |

## Step 2 — Choose the fields

```json
{
  "name": "Echo Nano",
  "sync_path": null,
  "filename_template": "{podcast}/{date} - {title}.{ext}",
  "max_filename_length": 128,
  "ascii_only": false,
  "preserve_structure": true
}
```

- **`name`** — matched **case-sensitively and exactly** by
  `:set-device-profile`. `HiBy R4` ≠ `Hiby R4`. Keep it short and typeable.
- **`sync_path`** — the device's target dir (e.g. `"X:\\Podcasts"` on Windows,
  where `X:` is the device's drive letter; note doubled backslashes in JSON). Leave `null` if the device isn't plugged
  in / the drive letter is unknown; the user sets it later via the Sync buffer
  `p` picker. When set, activating the profile auto-targets it.
- **`filename_template`** — see the token reference in
  `docs/DEVICE_PROFILES.md`. A literal `/` creates a device subfolder **only
  when `preserve_structure: true`**; in flat mode (`false`) separators in the
  rendered output are flattened to `_`. Common choices:
  - Rich player: `{podcast}/{date} - {title}.{ext}`
  - Ordered by episode: `{podcast}/{track:03} - {title}.{ext}`
  - Minimal: `{podcast}/{title}.{ext}`

## Step 3 — Locate config.json

`Config::default_config_path()` uses `directories::ProjectDirs` with
`("", "", "podcast-tui")`, so the path is:

| OS | Path |
|---|---|
| **Windows** | `%APPDATA%\podcast-tui\config\config.json` (note the nested `config\` subfolder) |
| **Linux** | `~/.config/podcast-tui/config.json` |
| **macOS** | `~/Library/Application Support/podcast-tui/config.json` |

## Step 4 — Edit safely

1. **Back up** first: copy `config.json` → `config.json.bak`.
2. **Append** the new object to the existing `device_profiles` array. Do **not**
   overwrite existing profiles or change `active_device_profile` unless the user
   asked to activate the new device.
3. Top-level `Config` fields (`audio`, `downloads`, `keybindings`, `storage`,
   `ui`) are **not** `#[serde(default)]`, so never hand-write a partial config —
   only edit the existing full file.

## Step 5 — Validate

1. Confirm the file is valid JSON and the new `name` has no hidden/trailing
   characters (check the byte length).
2. Launch the user's **actual** binary briefly and confirm it loads without
   rewriting/resetting the file, and that the new profile is present. The app
   only reads config **at startup**, so a running instance won't see the change
   until restart.
   - If the user runs from source: `cargo run --release`.
   - If installed: the winget binary.

## Gotchas (things that have bitten us)

- **App reads config only at startup.** After editing, the user must **restart**
  podcast-tui before `:set-device-profile <name>` will find the new profile.
  Otherwise it reports `No device profile named '<name>'`.
- **Case-sensitive exact name match.** A typo like `Hiby R4` vs `HiBy R4` yields
  "No device profile named…". Tip: type `:set-device-profile ` then **Tab** to
  cycle exact names.
- **`sync_path` auto-targeting is source-HEAD (unreleased at time of writing).**
  Released winget builds (≤ v1.15.0) silently ignore `sync_path`; the user sets
  the target manually via `p`. Confirm which binary the user runs before
  promising auto-target behavior.
- **MTP devices can't be synced over the cable.** If research shows MTP-only,
  tell the user to use a **card reader** for the microSD.
- **Flat layout disables orphan cleanup.** `preserve_structure: false` skips
  deletion of stale podcast files at the device root (they're indistinguishable
  from the user's own files). Only choose it for devices that truly can't browse
  subfolders.

## Reference devices (patterns)

| Device | Connection | `ascii_only` | `preserve_structure` | Template |
|---|---|---|---|---|
| Innioasis Y1 (budget, raw filename) | USB Mass Storage | `true` | `true` | `{podcast}/{track:03} - {title}.{ext}` |
| FiiO / HiBy R4 (rich, ID3, Unicode) | HiBy R4 = **MTP** (card reader) | `false` | `true` | `{podcast}/{date} - {title}.{ext}` |
| Snowsky Echo Nano (FiiO, folder-browsing) | **USB Mass Storage** (direct) | `false` | `true` | `{podcast}/{date} - {title}.{ext}` |

## Scope note
Adding a profile to a user's local `config.json` needs **no** repo change. Only
touch repo docs (`docs/DEVICE_PROFILES.md`, `CHANGELOG.md`) if the user wants to
contribute the device as a documented worked example via PR.
