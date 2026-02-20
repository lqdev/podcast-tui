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

```bash
grep '^version = ' Cargo.toml
git --no-pager tag --sort=-v:refname | head -5
```

Increment based on changes:
- **Major** (X.0.0): Breaking changes to config format, storage schema, or CLI behavior
- **Minor** (X.Y.0): New features, commands, or keybindings
- **Patch** (X.Y.Z): Bug fixes only

### 2. Finalize CHANGELOG.md

Use the `update-changelog` skill to split `[Unreleased]` into a versioned section:

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
<content from [Unreleased]>

### Fixed
<content from [Unreleased]>
```

Add a fresh empty `## [Unreleased]` section above it.

### 3. Update Cargo.toml version

```bash
# Update the version line (top-level [package] only)
# The post-release workflow also does this, but setting it now ensures
# local builds and `--version` output are correct for testing.
```

Edit the `version = "..."` line in `Cargo.toml` to the new version.

### 4. Run full verification

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

All four must pass. The release build is critical — it catches optimization-only issues.

### 5. Commit the release prep

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: prepare release vX.Y.Z

- Finalize CHANGELOG.md for vX.Y.Z
- Update Cargo.toml version to X.Y.Z"
```

### 6. Tag and push

```bash
git tag vX.Y.Z
git push origin main
git push origin vX.Y.Z
```

Pushing the tag triggers the `release.yml` workflow which:
1. Builds Linux (x86_64, aarch64) and Windows (x86_64, aarch64) binaries
2. Creates a GitHub Release with all artifacts
3. Runs smoke tests on the built binaries

### 7. Verify the release

After CI completes (~10 minutes):
- Check [Actions](https://github.com/lqdev/podcast-tui/actions) — all jobs green
- Check [Releases](https://github.com/lqdev/podcast-tui/releases) — new release with 4+ artifacts
- The `post-release-version-sync.yml` workflow auto-runs to sync `Cargo.toml` and generate winget manifests

### 8. Post-release verification

After `post-release-version-sync` completes:
- Verify `Cargo.toml` on `main` has the new version
- Verify `manifests/l/lqdev/PodcastTUI/<version>/` directory was created
- Pull latest `main` to stay in sync

## What NOT to do

- **Don't** update winget manifests manually — the post-release workflow handles it
- **Don't** edit `Cargo.toml` version in the release workflow — it does ephemeral version injection during builds
- **Don't** create the GitHub Release manually — the workflow uses `softprops/action-gh-release` with auto-generated release notes
- **Don't** skip the release build verification — `cargo build --release` catches issues that debug builds miss

## Naming convention for tags

Tags use clean semver with `v` prefix: `v1.6.0`, `v1.7.0`, `v1.8.0`, etc.

Artifact naming follows: `podcast-tui-v{VERSION}-{os}-{arch}.{ext}`
