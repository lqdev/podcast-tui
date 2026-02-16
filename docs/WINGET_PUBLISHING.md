# Windows Package Manager (winget) Publishing

This document covers how Podcast TUI is published to the [Windows Package Manager](https://learn.microsoft.com/en-us/windows/package-manager/) (winget) community repository, enabling users to install the application with a single command:

```powershell
winget install lqdev.PodcastTUI
```

## Overview

Podcast TUI is registered in the [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) community repository under the package identifier **`lqdev.PodcastTUI`**. The manifests are also maintained locally in the `manifests/` directory of this repository.

### Package Details

| Field | Value |
|-------|-------|
| **Package Identifier** | `lqdev.PodcastTUI` |
| **Publisher** | lqdev |
| **License** | MIT License |
| **Architectures** | x64, ARM64 |
| **Installer Type** | zip (portable) |
| **Package URL** | https://github.com/lqdev/podcast-tui |

---

## Manifest Structure

Winget uses a multi-file manifest format. Each version has three YAML files stored under `manifests/l/lqdev/PodcastTUI/<version>/`:

```
manifests/
└── l/
    └── lqdev/
        └── PodcastTUI/
            └── 1.5.0/
                ├── lqdev.PodcastTUI.yaml                 # Version manifest
                ├── lqdev.PodcastTUI.installer.yaml        # Installer manifest
                └── lqdev.PodcastTUI.locale.en-US.yaml     # Locale manifest
```

### Version Manifest (`lqdev.PodcastTUI.yaml`)

Contains top-level package identification:
- `PackageIdentifier` — unique package ID (`lqdev.PodcastTUI`)
- `PackageVersion` — semantic version (e.g., `1.5.0`)
- `DefaultLocale` — default language locale (`en-US`)
- `ManifestType` — set to `version`
- `ManifestVersion` — winget manifest schema version (e.g., `1.10.0`)

### Installer Manifest (`lqdev.PodcastTUI.installer.yaml`)

Describes how to download and install the application:
- **Installer type**: `zip` with `NestedInstallerType: portable`
- **Architecture entries**: One per supported architecture (x64, ARM64)
- **Installer URLs**: Point to GitHub Release zip archives
- **SHA256 hashes**: Integrity verification for each archive
- **Portable alias**: `PortableCommandAlias: podcast-tui` so the binary is available on PATH
- **Release date**: Date the version was published

Each architecture entry specifies:
```yaml
- Architecture: arm64
  NestedInstallerFiles:
  - RelativeFilePath: podcast-tui-v1.5.0-mvp-windows-aarch64\podcast-tui.exe
    PortableCommandAlias: podcast-tui
  InstallerUrl: https://github.com/lqdev/podcast-tui/releases/download/v1.5.0-mvp/...
  InstallerSha256: <sha256-hash>
```

### Locale Manifest (`lqdev.PodcastTUI.locale.en-US.yaml`)

Contains user-facing metadata:
- `PackageName`, `Publisher`, `ShortDescription`
- `PublisherUrl`, `PackageUrl`, `PublisherSupportUrl`
- `License`
- `ReleaseNotesUrl` — links to the GitHub release page
- `Documentations` — links to wiki/documentation

---

## Tools

### wingetcreate (Windows Package Manager Manifest Creator)

The manifests are generated and managed using [wingetcreate](https://github.com/microsoft/winget-create), a CLI tool that automates manifest creation, updates, and submission.

**Install wingetcreate:**

```powershell
winget install wingetcreate
```

Alternative installation methods:
- **GitHub Releases**: Download MSIX from the [winget-create releases](https://github.com/microsoft/winget-create/releases)
- **Standalone EXE**: Download from `https://aka.ms/wingetcreate/latest`

---

## Workflow

### Creating a New Version Manifest

When a new release of Podcast TUI is published on GitHub, follow these steps to update the winget package.

#### Step 1: Build and Publish the Release

1. Build release binaries using the [build system](BUILD_SYSTEM.md):
   ```powershell
   .\scripts\build-releases-windows.ps1
   ```
2. Create a GitHub Release with the version tag (e.g., `v1.6.0-mvp`)
3. Upload the zip archives for each architecture

#### Step 2: Create or Update the Manifest

**For a new version (update existing package):**

```powershell
wingetcreate update lqdev.PodcastTUI `
  --version 1.6.0 `
  --urls "https://github.com/lqdev/podcast-tui/releases/download/v1.6.0-mvp/podcast-tui-v1.6.0-mvp-windows-aarch64.zip|arm64" `
        "https://github.com/lqdev/podcast-tui/releases/download/v1.6.0-mvp/podcast-tui-v1.6.0-mvp-windows-x86_64.zip|x64" `
  --out manifests
```

The `|arm64` and `|x64` suffixes explicitly specify the architecture for each URL.

**For a brand-new package (first-time submission):**

```powershell
wingetcreate new `
  "https://github.com/lqdev/podcast-tui/releases/download/v1.5.0-mvp/podcast-tui-v1.5.0-mvp-windows-aarch64.zip" `
  "https://github.com/lqdev/podcast-tui/releases/download/v1.5.0-mvp/podcast-tui-v1.5.0-mvp-windows-x86_64.zip" `
  --out manifests
```

This launches an interactive prompt to fill in the package identifier, publisher, description, and other metadata.

#### Step 3: Validate the Manifest

Enable local manifest files (one-time setup):

```powershell
winget settings --enable LocalManifestFiles
```

Validate manifest schema:

```powershell
winget validate manifests\l\lqdev\PodcastTUI\1.6.0
```

#### Step 4: Test Locally

Install from the local manifest to verify everything works:

```powershell
winget install --manifest manifests\l\lqdev\PodcastTUI\1.6.0
```

Verify the installation:

```powershell
podcast-tui --help
```

Uninstall after testing:

```powershell
winget uninstall lqdev.PodcastTUI
```

#### Step 5: Submit to winget-pkgs

**Option A — Submit via wingetcreate (recommended):**

```powershell
wingetcreate submit manifests\l\lqdev\PodcastTUI\1.6.0 --token <github-pat>
```

This automatically:
1. Forks `microsoft/winget-pkgs` (if not already forked)
2. Creates a branch with the manifest files
3. Opens a pull request to the upstream repository

**Option B — Submit manually:**

1. Fork [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs)
2. Copy the manifest files to `manifests/l/lqdev/PodcastTUI/<version>/`
3. Commit and push to your fork
4. Open a PR to the upstream repository

#### Step 6: PR Review Process

After submission, the PR goes through:
1. **Automated validation** — Schema checks, hash verification, URL accessibility
2. **Automated testing** — The package may be test-installed in a sandbox
3. **Manual review** — Maintainers review for compliance with [winget-pkgs policies](https://github.com/microsoft/winget-pkgs/blob/master/AUTHORING_MANIFESTS.md)
4. **Merge** — Once approved, the package becomes available via `winget install`

---

## Local Manifest Management

### Directory Convention

Manifests are stored locally using the same directory structure as `winget-pkgs`:

```
manifests/l/lqdev/PodcastTUI/<version>/
```

This makes it easy to copy files directly into a `winget-pkgs` fork.

### Keeping Manifests in Sync

When publishing a new version:

1. Generate new manifest files with `wingetcreate update`
2. Copy them into `manifests/l/lqdev/PodcastTUI/<new-version>/`
3. Commit to the podcast-tui repo for tracking
4. Submit to `winget-pkgs` via `wingetcreate submit` or manually

### SHA256 Hash Generation

If you need to manually compute SHA256 hashes for installer URLs:

```powershell
# PowerShell
Get-FileHash .\podcast-tui-v1.6.0-mvp-windows-x86_64.zip -Algorithm SHA256
```

```bash
# Linux/macOS
sha256sum podcast-tui-v1.6.0-mvp-windows-x86_64.zip
```

---

## GitHub Personal Access Token

The `wingetcreate submit` and `wingetcreate update --submit` commands require a GitHub PAT with the following scopes:

- `public_repo` — to fork and create PRs on `winget-pkgs`

Generate a token at https://github.com/settings/tokens and pass it via `--token` or cache it:

```powershell
wingetcreate token --store <github-pat>
```

---

## Version History

| Version | Release Date | Notes |
|---------|-------------|-------|
| 1.5.0 | 2026-02-15 | Initial winget submission (x64 + ARM64) |

---

## Quick Reference

```powershell
# Install wingetcreate
winget install wingetcreate

# Update manifest for new version
wingetcreate update lqdev.PodcastTUI --version <VER> --urls "<url1>|<arch1>" "<url2>|<arch2>" --out manifests

# Validate manifest
winget validate manifests\l\lqdev\PodcastTUI\<VER>

# Test locally
winget install --manifest manifests\l\lqdev\PodcastTUI\<VER>

# Submit PR to winget-pkgs
wingetcreate submit manifests\l\lqdev\PodcastTUI\<VER> --token <PAT>
```

---

## References

- [wingetcreate documentation](https://github.com/microsoft/winget-create)
- [winget-pkgs repository](https://github.com/microsoft/winget-pkgs)
- [Authoring manifests guide](https://github.com/microsoft/winget-pkgs/blob/master/AUTHORING_MANIFESTS.md)
- [Manifest schema reference](https://aka.ms/winget-manifest.version.1.10.0.schema.json)
- [Windows Package Manager documentation](https://learn.microsoft.com/en-us/windows/package-manager/)
- [Build System documentation](BUILD_SYSTEM.md)
