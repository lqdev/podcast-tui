# ADR-004: Nix Flake for NixOS Packaging

**Status**: Accepted  
**Date**: 2026-03  
**Deciders**: @lqdev

## Context

NixOS users cannot install podcast-tui. Pre-built GitHub Release binaries are dynamically linked against glibc and ALSA at standard FHS paths (`/lib64/ld-linux-x86-64.so.2`, `libasound.so.2`) — paths that don't exist on NixOS, where everything lives under `/nix/store/`. There was no viable install path for NixOS users.

The application depends on `alsa-lib` (via rodio → cpal → alsa-sys) for built-in audio playback on Linux, which provides pause/resume, seek, volume control, and position tracking. An external player fallback exists but lacks these interactive features.

## Decision

Use a **Nix flake** (`flake.nix`) with **Crane** as the Rust builder to package podcast-tui for NixOS.

The flake provides:
- `packages.default` — the full binary with rodio audio support
- `apps.default` — enables `nix run github:lqdev/podcast-tui`
- `devShells.default` — development environment with cargo, rust-analyzer, and native deps
- `checks` — clippy and fmt validation

Native dependencies (`alsa-lib`, `pkg-config`) are declared in the flake and resolved by Nix, ensuring the binary works correctly on NixOS with full audio support.

## Alternatives Considered

### Static musl binary

Build with `x86_64-unknown-linux-musl` to produce a universally-compatible static binary.

**Rejected because**: rodio (ALSA) cannot be cleanly statically linked. This would require making rodio an optional dependency, which means NixOS users lose built-in audio features (pause, seek, volume, position tracking) — falling back to the external player subprocess which has no interactive controls.

### naersk (Nix Rust builder)

An alternative Nix builder for Rust.

**Rejected because**: Community activity is declining. Crane has better dependency caching (separates deps from source for faster rebuilds) and a more composable API.

### rustPlatform.buildRustPackage (nixpkgs standard)

The standard Rust builder used in the official nixpkgs repository.

**Not rejected, but deferred**: This is the required format for submitting to nixpkgs (the official NixOS package repository). A future PR to `NixOS/nixpkgs` will use `buildRustPackage`. The Crane flake provides immediate value without waiting for the nixpkgs review cycle.

## Consequences

### Positive
- NixOS users can install via `nix profile install github:lqdev/podcast-tui`
- Full rodio audio support (pause, seek, volume, position tracking)
- Contributors on NixOS get a ready-to-use dev environment via `nix develop`
- `Cargo.lock` is now tracked in git, fixing broken CI cache keys and enabling reproducible builds
- `nix run` enables zero-commitment trial without installation

### Negative
- New file (`flake.nix`) to maintain — flake inputs need periodic updates via `nix flake update`
- `flake.lock` must be generated on a Nix-capable system (cannot be generated on Windows)
- First build from source takes 5–15 minutes (no binary cache yet)

### Neutral
- macOS support is structurally available via `flake-utils.eachDefaultSystem` but `meta.platforms` is restricted to Linux until macOS builds are tested
- Committing `Cargo.lock` changes the dependency update workflow: must explicitly run `cargo update` instead of getting latest compatible versions automatically

---

*See [NIX_PACKAGING.md](../NIX_PACKAGING.md) for usage instructions and roadmap.*
