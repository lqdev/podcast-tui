# Nix Packaging for Podcast TUI

This document covers how to install, run, and develop podcast-tui on systems
that use the [Nix package manager](https://nixos.org/) — including NixOS, Home
Manager, and any Linux distro running Nix in single-user mode.

> **TL;DR** — Add the flake input, import the Home Manager module, set
> `programs.podcast-tui.enable = true`. Done. The default install is a
> sub-10-second download of a prebuilt binary from GitHub Releases (no source
> compile, no Cachix subscription, no `extra-substituters` opt-in).

---

## Why Nix Packaging?

Pre-built Linux binaries from GitHub Releases are dynamically linked against
glibc and ALSA at FHS paths (`/lib64/ld-linux-x86-64.so.2`,
`libasound.so.2`). NixOS uses `/nix/store/` instead, so those binaries fail to
run unless wrapped. The flake solves that two ways:

1. **`packages.podcast-tui-bin`** — fetches the GitHub Release tarball and
   patches the ELF interpreter/RPATH (`autoPatchelfHook`) so the upstream
   binary works natively on NixOS. This is the **default** on supported
   platforms.
2. **`packages.podcast-tui-source`** — full Crane build from source. Used as
   the fallback on platforms without a published binary (e.g. when a release
   hasn't been cut for `aarch64-linux` yet) and exercised by `nix flake
   check` on every commit.

`packages.default` automatically picks the right one for your system.

---

## Quick Install

### Try it (zero commitment)

```bash
nix run github:lqdev/podcast-tui
```

Pulls the prebuilt binary on `x86_64-linux` (~5 sec on a fast link). Falls
back to source compile (~5 min) on `aarch64-linux` until the next release.

### Install to your user profile

```bash
nix profile install github:lqdev/podcast-tui
podcast-tui --version

# Update later:
nix profile upgrade podcast-tui
```

---

## Declarative Install (Recommended)

Both modules expose a minimal, stable API:
`programs.podcast-tui.enable = true;` plus an optional `package` override.

### Home Manager (per-user)

In your `flake.nix`:

```nix
{
  inputs = {
    nixpkgs.url       = "github:NixOS/nixpkgs/nixos-unstable";
    home-manager.url  = "github:nix-community/home-manager";
    podcast-tui.url   = "github:lqdev/podcast-tui";
    # Optional but recommended — keeps closure size down by sharing nixpkgs:
    podcast-tui.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs: {
    homeConfigurations."you@host" = inputs.home-manager.lib.homeManagerConfiguration {
      pkgs = import inputs.nixpkgs { system = "x86_64-linux"; };
      modules = [
        inputs.podcast-tui.homeManagerModules.default
        ({ ... }: {
          programs.podcast-tui.enable = true;
        })
      ];
    };
  };
}
```

Then `home-manager switch`. `which podcast-tui` will resolve to your
HM-managed profile.

> **Home Manager rename note:** HM renamed `homeManagerModules` →
> `homeModules` in May 2025. Both names are exported and work — use whichever
> matches your HM version.

### NixOS (system-wide)

```nix
{
  inputs.podcast-tui.url = "github:lqdev/podcast-tui";
  inputs.podcast-tui.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { nixpkgs, podcast-tui, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        podcast-tui.nixosModules.default
        ({ ... }: {
          programs.podcast-tui.enable = true;
        })
        # ... your other modules
      ];
    };
  };
}
```

`sudo nixos-rebuild switch` and `podcast-tui` is on `$PATH` for everyone.

### Overlay (use anywhere `pkgs.X` works)

```nix
{
  nixpkgs.overlays = [ inputs.podcast-tui.overlays.default ];
}
```

You can now reference `pkgs.podcast-tui` in any module, devShell, or system
config without re-importing the flake.

---

## Update Workflow

The flake is a pull-based input — `nixos-rebuild switch` does **not** pull
new versions on its own. To update:

```bash
# Update everything
nix flake update
sudo nixos-rebuild switch    # or: home-manager switch

# Update only podcast-tui
nix flake lock --update-input podcast-tui
sudo nixos-rebuild switch
```

To pin to a specific version:

```nix
inputs.podcast-tui.url = "github:lqdev/podcast-tui/v1.12.0";
```

Each release ships an automated PR (`chore(nix): bump release hashes for
vX.Y.Z`) that updates `nix/release-hashes.nix` so `nix run` users immediately
get the prebuilt binary. Until that PR merges, `nix run` for an
unreleased-yet tag falls through to a source build.

---

## Supported Platforms

| System            | Default behavior                          | Source build available? |
|-------------------|-------------------------------------------|-------------------------|
| `x86_64-linux`    | Prebuilt binary from GitHub Releases      | Yes (Crane)             |
| `aarch64-linux`   | Prebuilt binary (after release v1.13.0+); else source build | Yes (Crane) |
| `x86_64-darwin`   | Not packaged — see roadmap                | No (untested)           |
| `aarch64-darwin`  | Not packaged — see roadmap                | No (untested)           |

---

## Development

### Dev shell

```bash
git clone https://github.com/lqdev/podcast-tui.git
cd podcast-tui
nix develop
```

Drops you into a shell with the Rust toolchain, `rust-analyzer`,
`cargo-watch`, `pkg-config`, and `alsa-lib`. Standard cargo commands work
inside.

### Direnv (optional)

```bash
echo 'use flake' > .envrc
direnv allow
```

Auto-activates the dev shell on `cd`.

### Available `nix build` targets

| Target                           | What you get                         |
|----------------------------------|--------------------------------------|
| `nix build .`                    | The default for your system (binary or source) |
| `nix build .#podcast-tui-bin`    | Force the prebuilt binary path       |
| `nix build .#podcast-tui-source` | Force the from-source Crane build    |
| `nix flake check`                | Build + clippy (`-D warnings`) + rustfmt |

---

## Troubleshooting

### "experimental Nix feature 'flakes' is disabled"

```nix
# /etc/nix/nix.conf or ~/.config/nix/nix.conf
experimental-features = nix-command flakes
```

### "error while loading shared libraries: libasound.so.2"

You ran a binary downloaded directly from GitHub Releases (FHS-linked).
Use `nix run`, `nix profile install`, or a flake input instead — those go
through the patched `podcast-tui-bin` derivation.

### Build is taking 5+ minutes

You're falling back to the source build because no prebuilt binary exists
for your system + version. Either pin to a tag that has a prebuilt binary,
or wait for the next release. The auto hash-bump PR lands within minutes
of each release publishing.

### `nix flake check` fails with "hash mismatch" after I bumped the version

Run `bash scripts/update-flake-hashes.sh vX.Y.Z` to regenerate
`nix/release-hashes.nix`. (CI does this automatically after each release;
this only matters if you're testing locally.)

---

## Audio on NixOS

podcast-tui uses rodio (cpal → ALSA). On NixOS:

- **PipeWire** users — works transparently via PipeWire's ALSA layer
- **PulseAudio** users — works transparently via Pulse's ALSA layer
- **Direct ALSA** users — works directly

`alsa-lib` is in the derivation's `buildInputs`, which is sufficient for all
three. If audio fails (e.g. headless), podcast-tui falls back to an
external player (mpv / vlc / ffplay).

---

## Roadmap

- **nixpkgs submission** — once stable, submit `pkgs.podcast-tui` to
  upstream nixpkgs so users get auto-updates with their channel. Tracked in
  the repo's issue list under the `nix` label.
- **macOS** — the source path is platform-agnostic; a `darwin` build needs
  testing + ALSA → CoreAudio audio backend changes.

---

## Architecture Deep-Dive

For contributors hacking on the flake itself, see
[`NIX_FLAKE_INTERNALS.md`](NIX_FLAKE_INTERNALS.md), which covers the
dual-output structure, hash-bump CI flow, and how `packages.default` routes
between binary and source.

The decision to ship a binary-fetch flake (rather than source-only +
Cachix) is recorded in
[ADR-005](adr/ADR-005-binary-fetch-flake.md).

---

*Last updated: 2026-05 · Version: v1.12.0 · Maintainer:
[@lqdev](https://github.com/lqdev)*
