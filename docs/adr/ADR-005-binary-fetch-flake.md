# ADR-005: Binary-Fetch Flake for NixOS Distribution

**Status**: Accepted
**Date**: 2026-05
**Deciders**: @lqdev
**Supersedes**: Builds on [ADR-004](ADR-004-nix-packaging.md) (which established the flake itself)

## Context

[ADR-004](ADR-004-nix-packaging.md) established the source-only Crane flake
that lets NixOS users install podcast-tui. In practice, that left two pain
points:

1. **Cold-cache install takes 5–15 minutes.** Every new user (and every
   user on a fresh machine) compiles the entire dependency tree from
   source. This is a steep first-impression tax for a TUI utility.
2. **Discoverability requires `nix flake update`.** Even after install,
   getting a new release means manually bumping `flake.lock` — there's no
   "channel updates" path until podcast-tui ships in nixpkgs.

We considered three ways to address (1):

- **Cachix** — host a binary cache. Free for OSS but requires user opt-in
  via `nixConfig.extra-substituters` (a UX wart that prompts an
  "untrusted substituter" warning) and adds operational surface (token
  management, bandwidth caps).
- **Garnix** — turnkey Nix CI + cache for GitHub. Free tier for small
  OSS projects but introduces a third-party CI dependency and the same
  `extra-substituters` opt-in.
- **Binary-fetch** — `pkgs.fetchurl` + `pkgs.autoPatchelfHook` against the
  GitHub Release tarballs we already publish. The binary itself becomes
  the cache; nothing extra to host or subscribe to.

The binary-fetch pattern is well-established in nixpkgs (vscode, slack,
zoom, jetbrains-*, every closed-source vendor binary) — it's the
standard answer when "compile from source via Nix" is too slow or
impossible.

## Decision

Adopt a **dual-output binary-fetch flake**:

- `packages.podcast-tui-bin` — `fetchurl` + `autoPatchelfHook` consuming
  the GitHub Release tarball for the current `system`.
- `packages.podcast-tui-source` — the existing Crane build from local
  source.
- `packages.default` — routes per system: returns the binary when a
  prebuilt is published for `system`, falls through to source on
  platforms without a binary, throws at eval time on unsupported
  platforms (Darwin).

A small `nix/release-hashes.nix` data file holds the per-platform
sha256s. CI auto-generates an "update hashes" PR after each release
(`scripts/update-flake-hashes.sh` invoked from
`.github/workflows/release.yml`), so the flake serves prebuilt binaries
within minutes of a release publishing.

We additionally export `homeManagerModules.default`,
`homeModules.default` (alias for HM's May-2025 rename), and
`nixosModules.default` so users can install with
`programs.podcast-tui.enable = true;` instead of manually adding
packages.

## Alternatives Considered

### Source-only + Cachix
**Rejected.** Adds a permanent operational dependency (Cachix
subscription, token, monitoring), forces users into the
`extra-substituters` workflow with its security warning, and yields an
identical end-user experience to binary-fetch. The only objective
upside is "philosophically purer" — and we already publish the same
binary as a release artifact, so the purity argument is moot.

### Source-only + Garnix
**Rejected** for the same reasons as Cachix, plus it locks us into a
specific CI provider whose free-tier ceiling (build-minutes) becomes a
ticking clock as the project grows.

### Submit to nixpkgs and stop maintaining the flake
**Deferred, not rejected.** Submitting to nixpkgs is the right
long-term answer for discoverability and channel-based updates, and is
tracked as a follow-up issue. But nixpkgs has a multi-week review
cycle, requires porting away from Crane (nixpkgs prefers
`rustPlatform.buildRustPackage`), and doesn't help users on
`nixos-stable` channels until the next channel rotation. The flake is
how we get *immediate* install paths; nixpkgs is how we get
*automatic* ones. We need both.

### Inline the hash table in `flake.nix`
**Rejected.** Keeping `release-hashes.nix` as a dedicated data file
makes the auto-bump PR a tiny mechanical diff with zero risk of
clobbering flake logic. It also makes the table trivially hand-auditable.

## Consequences

### Positive
- **Sub-10-second install** on supported platforms (just a tarball
  download + ELF patching) instead of a 5–15 min source compile.
- **Zero extra infra** — no Cachix, no Garnix, no `extra-substituters`,
  no tokens. The GitHub Release we already publish *is* the cache.
- **Standard nixpkgs pattern** — anyone familiar with how vscode or
  slack are packaged in nixpkgs immediately understands our flake.
- **Transparent fallback** — `packages.default` graceful-degrades to
  source on platforms without a published binary, so the flake never
  silently breaks for ARM users or pre-release commits.
- **Clean module API** — `programs.podcast-tui.enable = true;` is the
  sole user touchpoint, matching the nixvim/helix pattern.

### Negative
- **Binary-only by default on Linux.** Purist users who prefer
  "everything from source" can opt into `podcast-tui-source`
  explicitly via `nix build github:lqdev/podcast-tui#podcast-tui-source`
  (or pin their module to it). The `meta.sourceProvenance =
  [ binaryNativeCode ]` flag we set on the binary derivation is
  informational — it tells nixpkgs tooling this output is prebuilt
  binary code rather than built-from-source, but it does *not*
  trigger `nixpkgs.config.allowUnfree` (we are MIT-licensed).
- **Hash maintenance burden.** Releases now have an extra step (the
  auto-bump PR). This is fully automated, but if CI breaks, releases
  ship without prebuilt-binary support until the bump PR is opened
  manually. Mitigation: source build still works as a fallback during
  any CI outage.
- **Two derivations to maintain.** The Crane source build now exists
  primarily for `nix flake check` and the eventual nixpkgs
  submission. Some duplication of `commonArgs` between
  `mkSourceFor` and `checks` — acceptable trade-off for the safety net.

### Neutral
- **macOS still unsupported.** The binary-fetch path can't help here
  (no macOS release tarball today), and neither could the source-only
  path (untested on Darwin). Status quo from ADR-004.

## Implementation Notes

- The flake must remain backward-compatible: `nix run
  github:lqdev/podcast-tui` continues to work, `nix profile install`
  continues to work, no breaking changes to `packages.default`'s
  contract.
- The auto hash-bump PR uses `WINGET_SUBMIT_TOKEN` (re-purposed from
  the existing winget submission flow). A dedicated bot token would
  be cleaner; deferred until we hit a permission boundary.
- All new files live under `nix/` (modules + hash table) and
  `scripts/` (update script) — kept out of the repo root.

## References

- [ADR-004: Nix Flake for NixOS Packaging](ADR-004-nix-packaging.md)
- [`docs/NIX_PACKAGING.md`](../NIX_PACKAGING.md) — user-facing install docs
- [`docs/NIX_FLAKE_INTERNALS.md`](../NIX_FLAKE_INTERNALS.md) — contributor architecture deep-dive
- [nixpkgs `vscode` derivation](https://github.com/NixOS/nixpkgs/blob/master/pkgs/applications/editors/vscode/vscode.nix) — exemplar for the binary-fetch + `autoPatchelfHook` pattern
- [`helix` flake](https://github.com/helix-editor/helix/blob/master/flake.nix) — exemplar for the overlay + module export pattern
- [`nixvim` flake](https://github.com/nix-community/nixvim/blob/main/flake.nix) — exemplar for the `self`-as-first-arg module pattern
