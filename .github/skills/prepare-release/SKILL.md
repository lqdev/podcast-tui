---
name: prepare-release
description: Prepare a new release by finalizing CHANGELOG, tagging, and verifying builds. Covers everything before pushing the tag that triggers CI.
---

# Skill: Prepare a Release

## When to use
When `[Unreleased]` in CHANGELOG.md has accumulated features/fixes and you're ready to cut a new version.

## Prerequisites
- All intended PRs are merged to `main`
- `cargo test` and `cargo clippy -- -D warnings` pass on `main`
- You know the next version number (follow [Semantic Versioning](https://semver.org/))

## Steps

### 1. Determine the version number

Check the current version in `Cargo.toml` and the latest git tag:

```powershell
# Windows
Select-String '^version = ' Cargo.toml
git --no-pager tag --sort=-v:refname | Select-Object -First 5
```

Also check the **GitHub Releases page** (`https://github.com/lqdev/podcast-tui/releases`) to confirm the latest published release — the Cargo.toml version may lag behind due to the post-release workflow.

Increment based on changes in `[Unreleased]`:
- **Major** (X.0.0): Breaking changes to config format, storage schema, or CLI behavior
- **Minor** (X.Y.0): New features, commands, or keybindings
- **Patch** (X.Y.Z): Bug fixes only

### 2. Finalize CHANGELOG.md

Convert `[Unreleased]` into a versioned release section with today's date:

```markdown
## [Unreleased]

---

## [X.Y.Z] - YYYY-MM-DD

### Fixed
<content from [Unreleased] Fixed sections>

### Added
<content from [Unreleased] Added sections>
```

Rules:
- Add a **fresh empty `## [Unreleased]`** section at the top (above the new versioned section)
- Keep the `---` separator between the new section and the previous versioned entry
- Preserve all existing content — just move it from `[Unreleased]` to `[X.Y.Z]`
- `Fixed` sections conventionally come before `Added` within a release entry
- Remove the subtitle from the `[Unreleased]` heading (e.g. `- Playlist Enhancements (post-v1.6.0)` → just `## [Unreleased]`)

### 3. Update Cargo.toml version

Edit the `version = "..."` line in `Cargo.toml` to the new version (top-level `[package]` only):

```toml
version = "X.Y.Z"
```

> **Note**: The `post-release-version-sync.yml` workflow also updates this after publishing, but setting it locally ensures `cargo build --version` output is correct for verification.

### 4. Run full verification

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

All three must pass cleanly before tagging. The `test_opml_local_file` integration test has a pre-existing failure (missing local fixture file) — this is expected and can be ignored.

> `cargo build --release` is optional on Windows since CI runs the official release builds. Run it if you want extra confidence (it takes ~5 minutes).

### 5. Commit the release prep

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: prepare release vX.Y.Z

- Finalize CHANGELOG.md for vX.Y.Z
- Update Cargo.toml version to X.Y.Z

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### 6. Tag and push

```bash
git tag vX.Y.Z
git push origin main
git push origin vX.Y.Z
```

Pushing the tag triggers the `release.yml` workflow which:
1. Builds Linux (x86_64, aarch64) and Windows (x86_64, aarch64) binaries
2. Creates and **publishes** a GitHub Release with all artifacts and auto-generated release notes

### 7. Verify the release workflow

After pushing the tag:
```powershell
gh run list --workflow release.yml --limit 3
```

After CI completes (~10 minutes):
- Check [Actions](https://github.com/lqdev/podcast-tui/actions) — all jobs green
- Check [Releases](https://github.com/lqdev/podcast-tui/releases) — new release with 4 artifacts (Linux x86_64, Linux aarch64, Windows x86_64, Windows aarch64)

### 8. Post-release workflow

The `post-release-version-sync.yml` workflow triggers **automatically** when the GitHub Release is published (triggered by `release: types: [published]`, not by the tag push directly). It:
1. Updates `Cargo.toml` version on `main` (no-op if you already set it in step 3)
2. Generates winget manifests under `manifests/l/lqdev/PodcastTUI/<version>/`
3. Commits and pushes both to `main`

Monitor it:
```powershell
gh run list --workflow post-release-version-sync.yml --limit 3
```

After it completes:
```bash
git pull origin main   # pick up the post-release commit
```

Verify:
- `Cargo.toml` on `main` has the new version
- `manifests/l/lqdev/PodcastTUI/<version>/` directory exists

## Workflow trigger chain

```
git push origin vX.Y.Z
  └─► release.yml (tag push: v*)
        └─► builds binaries + creates GitHub Release (published)
              └─► post-release-version-sync.yml (release: published)
                    └─► updates Cargo.toml + generates winget manifests
```

## What NOT to do

- **Don't** update winget manifests manually — the post-release workflow handles it
- **Don't** create the GitHub Release manually — `release.yml` uses `softprops/action-gh-release` with auto-generated notes
- **Don't** skip `cargo clippy -- -D warnings` — it's required to pass before tagging

## Naming convention for tags

Tags use clean semver with `v` prefix: `v1.6.0`, `v1.7.0`, `v1.8.0`, `v1.9.0`, etc.

Artifact naming follows: `podcast-tui-v{VERSION}-{os}-{arch}.{ext}`
- `podcast-tui-v1.9.0-linux-x86_64.tar.gz`
- `podcast-tui-v1.9.0-linux-aarch64.tar.gz`
- `podcast-tui-v1.9.0-windows-x86_64.zip`
- `podcast-tui-v1.9.0-windows-aarch64.zip`
