# Nix Packaging for Podcast TUI

This document covers how podcast-tui is packaged for NixOS using a Nix flake, including installation methods, development workflows, and future roadmap.

## Overview

Podcast TUI provides a `flake.nix` that uses [Crane](https://github.com/ipetkov/crane) (a Nix Rust builder) to build the application with full rodio audio support. This enables NixOS users to install, run, and develop podcast-tui without manually managing native dependencies like ALSA.

### Why Nix Packaging?

Pre-built Linux binaries from GitHub Releases are dynamically linked against glibc and ALSA at standard paths (`/lib64/ld-linux-x86-64.so.2`, `libasound.so.2`). NixOS uses `/nix/store/` instead of the Filesystem Hierarchy Standard (FHS), so these binaries fail to run. The flake builds podcast-tui from source with Nix-managed dependencies, producing a binary that works natively on NixOS.

---

## Installation

### Try It (zero commitment)

```bash
nix run github:lqdev/podcast-tui
```

Downloads, builds, and runs podcast-tui in one command. Nothing is installed.

### Install to Profile (imperative)

```bash
nix profile install github:lqdev/podcast-tui
podcast-tui  # Now available on PATH
```

Update later:

```bash
nix profile upgrade github:lqdev/podcast-tui
```

### NixOS System Configuration (declarative)

Add podcast-tui as a flake input in your system `flake.nix`:

```nix
{
  inputs.podcast-tui.url = "github:lqdev/podcast-tui";

  outputs = { nixpkgs, podcast-tui, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [{
        environment.systemPackages = [
          podcast-tui.packages.x86_64-linux.default
        ];
      }];
    };
  };
}
```

Apply with `nixos-rebuild switch`.

### Home Manager (per-user declarative)

```nix
home.packages = [
  inputs.podcast-tui.packages.x86_64-linux.default
];
```

---

## Development

### Prerequisites

- [Nix](https://nixos.org/download.html) with flakes enabled
- Git

### Development Shell

```bash
git clone https://github.com/lqdev/podcast-tui.git
cd podcast-tui
nix develop
```

This drops you into a shell with:
- Rust toolchain (cargo, rustc, rustfmt, clippy)
- rust-analyzer (for IDE integration)
- cargo-watch (for file watching)
- pkg-config and alsa-lib (native dependencies)

All build commands work inside the dev shell:

```bash
cargo build          # Debug build
cargo build --release  # Release build
cargo test           # Run tests
cargo clippy         # Lint
cargo fmt            # Format
```

### Direnv Integration (optional)

For automatic environment activation when entering the project directory:

1. Install [direnv](https://direnv.net/) and [nix-direnv](https://github.com/nix-community/nix-direnv)
2. Create `.envrc` in the project root:
   ```
   use flake
   ```
3. Run `direnv allow`

The Nix development environment will activate automatically on `cd`.

---

## How the Flake Works

### Architecture

The `flake.nix` uses three key inputs:
- **nixpkgs** — the Nix package collection (provides alsa-lib, pkg-config, Rust toolchain)
- **crane** — a composable Nix Rust builder with superior dependency caching
- **flake-utils** — helper for multi-platform output generation

### Build Process

1. **Source filtering**: `craneLib.cleanCargoSource` strips non-Rust files to avoid unnecessary rebuilds
2. **Dependency caching**: `craneLib.buildDepsOnly` builds and caches dependencies separately from source code — deps change rarely, so subsequent builds are fast
3. **Full build**: `craneLib.buildPackage` builds the final binary using cached dependency artifacts
4. **Native deps**: `alsa-lib` (runtime) and `pkg-config` (build-time) are injected by Nix

### Outputs

| Output | Description |
|--------|-------------|
| `packages.default` | The built podcast-tui binary |
| `apps.default` | Enables `nix run` |
| `devShells.default` | Development environment |
| `checks.podcast-tui` | Build check |
| `checks.podcast-tui-clippy` | Clippy lint check |
| `checks.podcast-tui-fmt` | Format check |

### Why Crane?

| Builder | Pros | Cons |
|---------|------|------|
| **Crane** ✅ | Best caching (dep/source split), composable, active community | External flake input |
| naersk | Simple API | Declining activity, weaker caching |
| rustPlatform.buildRustPackage | Standard for nixpkgs | No dep caching, slower iteration |

See [ADR-004](adr/ADR-004-nix-packaging.md) for the full decision record.

---

## Updating Dependencies

### Nix inputs (nixpkgs, Crane)

```bash
nix flake update
git add flake.lock
git commit -m "chore: update Nix flake inputs"
```

### Cargo dependencies

```bash
cargo update
git add Cargo.lock
git commit -m "chore: update Cargo dependencies"
```

---

## Testing the Nix Build Locally

```bash
# Build the package
nix build .

# Verify the binary runs
./result/bin/podcast-tui --version

# Run checks (clippy, fmt)
nix flake check

# Test the dev shell
nix develop -c cargo --version
```

---

## Audio on NixOS

podcast-tui uses rodio (via cpal → ALSA) for audio playback. On NixOS:

- **PipeWire users**: PipeWire provides an ALSA compatibility layer. Audio works transparently.
- **PulseAudio users**: PulseAudio also provides ALSA compatibility. Audio works transparently.
- **Direct ALSA users**: Audio works directly.

The flake includes `alsa-lib` as a build dependency, which is sufficient for all three scenarios. No additional audio configuration is needed.

If built-in audio fails (e.g., in a headless environment), podcast-tui automatically falls back to an external player (mpv, vlc, or ffplay). You can also force external player usage via the `external_player` config option.

---

## Troubleshooting

### "error: experimental Nix feature 'flakes' is disabled"

Enable flakes in your Nix configuration:

```nix
# /etc/nix/nix.conf or ~/.config/nix/nix.conf
experimental-features = nix-command flakes
```

### "error while loading shared libraries: libasound.so.2"

This means the binary was built without Nix (e.g., downloaded from GitHub Releases). Use one of the Nix installation methods above instead.

### Build takes a long time

The first build compiles all dependencies from source (~5–15 minutes). Subsequent builds use Crane's dependency cache and are much faster. A future enhancement is setting up a Cachix binary cache.

### "flake.lock not found"

The `flake.lock` must be generated on a Nix-capable system:

```bash
nix flake lock
git add flake.lock
git commit -m "chore: add flake.lock"
```

---

## Roadmap

### Cachix Binary Cache
Set up [Cachix](https://www.cachix.org/) to provide pre-built binaries. This would eliminate the initial build time — users would download the cached binary instead of building from source.

### nixpkgs Submission
Submit podcast-tui to the official [NixOS/nixpkgs](https://github.com/NixOS/nixpkgs) repository using `rustPlatform.buildRustPackage`. This would make podcast-tui available via `pkgs.podcast-tui` without needing to add a flake input. Requires a PR and review cycle.

### Static musl Binary
Add a `x86_64-unknown-linux-musl` build target to CI for a universally-compatible static binary. This would be a "lite" option (external player audio only, no rodio) but would work on any Linux distribution including NixOS without Nix.

### CI Integration
Add `nix build` and `nix flake check` steps to the GitHub Actions CI workflow using [cachix/install-nix-action](https://github.com/cachix/install-nix-action). This ensures Nix packaging doesn't regress when dependencies or code change.

### macOS Support
The flake structure supports macOS via `flake-utils.eachDefaultSystem`, but `meta.platforms` is currently restricted to Linux. macOS support can be added once builds are tested on darwin.

---

*Last Updated: March 2026 | Version: v1.11.0 | Maintainer: [@lqdev](https://github.com/lqdev)*
