# Nix Flake Internals — Contributor Guide

> **Audience:** contributors hacking on `flake.nix`, the `nix/` modules, the
> hash-bump CI flow, or release tooling. End users should read
> [`NIX_PACKAGING.md`](NIX_PACKAGING.md) instead.

---

## File Layout

| Path                                   | Purpose                                  |
|----------------------------------------|------------------------------------------|
| `flake.nix`                            | Top-level flake outputs (packages, modules, overlay, dev shell, checks) |
| `nix/release-hashes.nix`               | Per-platform sha256 table for prebuilt tarballs (data; auto-regenerated) |
| `nix/hm-module.nix`                    | Home Manager module (`programs.podcast-tui.enable`) |
| `nix/nixos-module.nix`                 | NixOS module (`programs.podcast-tui.enable`) |
| `scripts/update-flake-hashes.sh`       | Regenerates `release-hashes.nix` from a tag's GH Release artifacts |
| `.github/workflows/release.yml`        | Builds tarballs **and** opens the auto hash-bump PR |

---

## Dual-Output Architecture

The flake exposes **three** package outputs per system:

```
packages.<system>.podcast-tui-bin    # autoPatchelf'd GH Release tarball
packages.<system>.podcast-tui-source # crane build from local source
packages.<system>.default            # alias to whichever exists for this system
```

`packages.default` resolves in this order:

1. **`podcast-tui-bin`** if `nix/release-hashes.nix` has a non-null hash for
   the current `system` and the `system` is in `binarySystems`.
2. **`podcast-tui-source`** if the `system` is in `sourceSystems` (currently
   `x86_64-linux` and `aarch64-linux`).
3. Otherwise, an `eval`-time `throw` — this guarantees Darwin users get a
   clear error at evaluation time rather than a silent failure mid-build.

This routing keeps the user-facing API minimal (everyone uses `default`)
while letting us surface both options for debugging or for downstream
packagers who want the "real" source build (e.g. a future nixpkgs
submission would consume `podcast-tui-source` directly).

---

## The Binary-Fetch Derivation

```nix
pkgs.stdenv.mkDerivation {
  src = pkgs.fetchurl { url = "...releases/download/vX.Y.Z/...tar.gz"; sha256 = ...; };
  nativeBuildInputs = [ pkgs.autoPatchelfHook ];
  buildInputs       = [ pkgs.alsa-lib pkgs.stdenv.cc.cc.lib ];
  installPhase = ''
    install -Dm755 podcast-tui $out/bin/podcast-tui
    ...
  '';
  meta.sourceProvenance = [ binaryNativeCode ];
}
```

Three pieces are doing all the work:

1. **`fetchurl`** with a fixed sha256 — Nix verifies the tarball is exactly
   what we expect and caches it in the binary cache like any other
   fixed-output derivation.
2. **`autoPatchelfHook`** — rewrites the ELF interpreter
   (`/lib64/ld-linux-x86-64.so.2` → `/nix/store/...glibc.../ld-linux...`)
   and RPATH so dynamically loaded libraries (`libasound.so.2`,
   `libstdc++.so.6`) resolve from `/nix/store` instead of FHS paths.
3. **`buildInputs`** — supplies the libraries `autoPatchelfHook` will
   actually patch in. `alsa-lib` for audio; `stdenv.cc.cc.lib` for libstdc++
   (rodio links against it via cpal).

`meta.sourceProvenance = [ binaryNativeCode ]` is the standard nixpkgs
marker for "this is a vendored binary, not a from-source build" — required
hygiene for any future nixpkgs PR.

---

## Hash Lifecycle

### Adding a new release

1. Push tag `vX.Y.Z`.
2. `release.yml` builds tarballs for `linux-x86_64` + `linux-aarch64` (and
   Windows, untouched by the flake) and uploads them to the GitHub Release.
3. The `update-flake-hashes` job runs **after** `create-release` succeeds,
   sleeps 30 seconds for asset propagation, then runs
   `scripts/update-flake-hashes.sh vX.Y.Z`.
4. The script `curl`s each `.sha256` sidecar, converts to SRI form via
   `nix hash convert`, and writes a fresh `nix/release-hashes.nix`.
5. The job opens a PR titled
   `chore(nix): bump release hashes for vX.Y.Z`. Merge it (CI green is the
   only gate) and `nix run github:lqdev/podcast-tui` immediately serves the
   prebuilt binary instead of source-building.

### Adding a new platform

1. Add a build job in `release.yml` (matrix entry under `build-linux` for a
   new Linux arch, or a new top-level job for non-Linux).
2. Add the system → platform-suffix mapping in three places:
   - `flake.nix` → `binarySystems` list and `platformTagFor`
   - `scripts/update-flake-hashes.sh` → `PLATFORMS` associative array
   - `release.yml` → `test-builds` matrix
3. If the new platform also gets a source build, add it to `sourceSystems`
   in `flake.nix` (and ensure crane supports it).
4. Cut a release. The auto hash-bump PR will populate the new platform's
   hash on first use.

### Why a separate data file (`release-hashes.nix`)?

Keeping the hash table in a Nix data file (rather than inline in
`flake.nix`) means the hash-bump PR has a tiny, mechanical diff — no risk
of CI accidentally rewriting flake logic. It also makes the table
trivial to audit by hand.

---

## Module Pattern (HM + NixOS)

Both modules follow the **`self`-as-first-arg** pattern from `nixvim` /
`helix`:

```nix
# nix/hm-module.nix
self: { config, lib, pkgs, ... }:
  let cfg = config.programs.podcast-tui;
  in  { ... config = lib.mkIf cfg.enable { home.packages = [ cfg.package ]; }; }

# flake.nix
homeManagerModules.default = import ./nix/hm-module.nix self;
```

Closing over `self` lets the module default `cfg.package` to
`self.packages.${pkgs.system}.default` without forcing the user to plumb
`extraSpecialArgs`. This is the modern best practice — older docs that show
`extraSpecialArgs = { inherit my-flake; }` are pre-2024 and unnecessary.

`homeModules.default` is exported as an alias of `homeManagerModules.default`
to match HM's May-2025 rename.

---

## Source Build (Crane)

The source path is unchanged from the pre-binary-fetch flake — Crane with
dependency caching, ALSA + pkg-config in `buildInputs`, source filtered to
Cargo + `*.opml` files (the latter for integration test fixtures).

It exists for two reasons:

1. **`nix flake check`** runs clippy + rustfmt + the full build on every
   commit, catching regressions that the binary path can't (since the
   binary is opaque).
2. **Fallback** for platforms / versions without a prebuilt binary.

If we ever submit to nixpkgs, the upstream packaging will consume this
source path (ported to `rustPlatform.buildRustPackage` to satisfy nixpkgs
review preferences).

---

## Local Testing Cheatsheet

```bash
# Eval-only check (fast, no builds)
nix flake check --no-build

# Build the prebuilt-binary path
nix build .#podcast-tui-bin
./result/bin/podcast-tui --version

# Force the from-source path
nix build .#podcast-tui-source

# Confirm `default` resolves to the binary on supported systems
[[ "$(nix path-info .#default --derivation)" \
   == "$(nix path-info .#podcast-tui-bin --derivation)" ]] && echo OK

# Test the hash-bump script
bash scripts/update-flake-hashes.sh v1.12.0
git diff nix/release-hashes.nix    # should be empty (idempotent)

# Lint the workflow
nix-shell -p actionlint --run "actionlint .github/workflows/release.yml"
```

---

## Common Failure Modes

| Symptom                                                 | Likely cause                                                  |
|---------------------------------------------------------|---------------------------------------------------------------|
| `hash mismatch in fixed-output derivation`              | Tarball was re-uploaded after the hash was committed; rerun the script. |
| `cannot find -lasound` (source build)                   | `alsa-lib` missing from `buildInputs` — check that crane/cargo build also has `pkg-config` in `nativeBuildInputs`. |
| `error: file 'nix/release-hashes.nix' does not exist` (during `nix build`) | New file not staged in git; `git add nix/`. The flake reads from the git tree, not the working directory. |
| `getAttr: attribute 'x86_64-darwin' missing`            | User on Darwin — they hit the `throw` in `packages.default`. Expected; we don't ship Darwin (yet). |
| Auto hash-bump PR not opened                            | Check the `update-flake-hashes` job logs — the most common cause is `WINGET_SUBMIT_TOKEN` lacking `pull-requests:write`. |

---

## Why this architecture?

See [ADR-005](adr/ADR-005-binary-fetch-flake.md) for the decision record.
Short version: source-only Nix builds either compile from scratch every
time (5–15 min, painful for casual users) or require a Cachix subscription
(operational complexity, free-tier ceilings, `extra-substituters` UX wart).
Consuming the GitHub Release tarball we already publish is dramatically
simpler, costs nothing extra, and is the standard nixpkgs pattern for
upstream projects that don't (yet) build natively under nixpkgs (vscode,
slack, zoom, jetbrains-*, etc.).

---

*Last updated: 2026-05*
