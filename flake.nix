{
  description = "Podcast TUI — a cross-platform terminal podcast manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    let
      # Systems with a published prebuilt binary tarball on GitHub Releases.
      # `packages.default` returns the binary on these systems and falls back
      # to a source build elsewhere. Keep in sync with `nix/release-hashes.nix`
      # and `.github/workflows/release.yml`.
      binarySystems = [ "x86_64-linux" "aarch64-linux" ];

      # Systems where the source build is exercised (`nix flake check`,
      # `nix build .#podcast-tui-source`). Crane supports both Linux arches.
      sourceSystems = [ "x86_64-linux" "aarch64-linux" ];

      # All systems for which we expose per-system outputs at all. Includes
      # Darwin so that `packages.default` and the overlay produce a *helpful*
      # eval-time error on macOS instead of a generic missing-attribute one.
      # Keep this a superset of binarySystems ∪ sourceSystems.
      allSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      releaseHashes = import ./nix/release-hashes.nix;

      # Map nix system -> tarball platform tag used in release filenames.
      platformTagFor = system: {
        "x86_64-linux"  = "linux-x86_64";
        "aarch64-linux" = "linux-aarch64";
      }.${system} or null;

      # Build the binary derivation for `system` if a hash is published, else null.
      mkBinaryFor = pkgs: system:
        let
          hash = releaseHashes.hashes.${system} or null;
          tag  = platformTagFor system;
        in
        if hash == null || tag == null then null else
        pkgs.stdenv.mkDerivation rec {
          pname = "podcast-tui";
          version = releaseHashes.version;

          src = pkgs.fetchurl {
            url = "https://github.com/lqdev/podcast-tui/releases/download/v${version}/podcast-tui-v${version}-${tag}.tar.gz";
            sha256 = hash;
          };

          nativeBuildInputs = [ pkgs.autoPatchelfHook ];
          buildInputs = [ pkgs.alsa-lib pkgs.stdenv.cc.cc.lib ];

          sourceRoot = "podcast-tui-v${version}-${tag}";

          dontConfigure = true;
          dontBuild = true;

          installPhase = ''
            runHook preInstall
            install -Dm755 podcast-tui $out/bin/podcast-tui
            for doc in README.md LICENSE CHANGELOG.md; do
              [ -f "$doc" ] && install -Dm644 "$doc" "$out/share/doc/podcast-tui/$doc" || true
            done
            runHook postInstall
          '';

          meta = with pkgs.lib; {
            description = "A cross-platform terminal user interface for podcast management (prebuilt binary)";
            homepage = "https://github.com/lqdev/podcast-tui";
            license = licenses.mit;
            platforms = binarySystems;
            mainProgram = "podcast-tui";
            sourceProvenance = with sourceTypes; [ binaryNativeCode ];
          };
        };

      # Build the source derivation for `system` (crane). Returns null on
      # systems we don't try to source-build (none today, but kept for
      # forward compat — e.g. macOS would land here).
      mkSourceFor = system:
        if !(builtins.elem system sourceSystems) then null else
        let
          pkgs = nixpkgs.legacyPackages.${system};
          craneLib = crane.mkLib pkgs;

          # Filter source to Rust/Cargo-relevant files plus OPML fixtures used
          # by integration tests.
          src = pkgs.lib.cleanSourceWith {
            src = craneLib.path ./.;
            filter = path: type:
              (craneLib.filterCargoSources path type) ||
              (builtins.match ".*\\.opml$" path != null);
          };

          commonArgs = {
            inherit src;
            strictDeps = true;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.alsa-lib ];
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          meta = with pkgs.lib; {
            description = "A cross-platform terminal user interface for podcast management (built from source)";
            homepage = "https://github.com/lqdev/podcast-tui";
            license = licenses.mit;
            platforms = platforms.linux;
            mainProgram = "podcast-tui";
          };
        });
    in
    {
      # ─── Cross-system flake outputs ───────────────────────────────────────
      # Overlay: makes `pkgs.podcast-tui` available to downstream Nix code.
      # Guarded so that on systems without a published default (e.g. Darwin),
      # importing the overlay still succeeds; only *referencing* `pkgs.podcast-tui`
      # surfaces the unsupported-system error.
      overlays.default = final: _prev:
        if (self.packages ? ${final.system}) && (self.packages.${final.system} ? default)
        then { podcast-tui = self.packages.${final.system}.default; }
        else { podcast-tui = throw "podcast-tui: no package available for ${final.system}"; };

      # Home Manager module: `programs.podcast-tui.enable = true;`
      homeManagerModules.default = import ./nix/hm-module.nix self;
      # Alias for the HM rename (post-2025-05) — both names resolve to the same module.
      homeModules.default = self.homeManagerModules.default;

      # NixOS module: system-wide install via `programs.podcast-tui.enable`.
      nixosModules.default = import ./nix/nixos-module.nix self;
    }
    //
    # ─── Per-system outputs ───────────────────────────────────────────────
    flake-utils.lib.eachSystem allSystems (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        bin    = mkBinaryFor pkgs system;
        source = mkSourceFor system;
        # `default` prefers the prebuilt binary; falls through to source if no
        # binary is published for this system. On systems with neither (e.g.
        # macOS today) the throw fires the moment something forces `default`,
        # giving users a clear, actionable error instead of an opaque
        # missing-attribute failure.
        hasDefault = bin != null || source != null;
        default =
          if bin != null then bin
          else if source != null then source
          else throw "podcast-tui: no prebuilt binary or source build available for ${system}. Supported: ${toString (nixpkgs.lib.unique (binarySystems ++ sourceSystems))}.";
      in
      {
        packages =
          (pkgs.lib.optionalAttrs hasDefault { inherit default; })
          // (pkgs.lib.optionalAttrs (bin    != null) { podcast-tui-bin    = bin; })
          // (pkgs.lib.optionalAttrs (source != null) { podcast-tui-source = source; });

        apps = pkgs.lib.optionalAttrs hasDefault {
          default = flake-utils.lib.mkApp { drv = default; };
        };

        devShells.default = (crane.mkLib pkgs).devShell {
          inputsFrom = pkgs.lib.optional (source != null) source;
          packages = with pkgs; [ rust-analyzer cargo-watch ];
        };

        checks = pkgs.lib.optionalAttrs (source != null) (
          let
            craneLib = crane.mkLib pkgs;
            src = pkgs.lib.cleanSourceWith {
              src = craneLib.path ./.;
              filter = path: type:
                (craneLib.filterCargoSources path type) ||
                (builtins.match ".*\\.opml$" path != null);
            };
            commonArgs = {
              inherit src;
              strictDeps = true;
              nativeBuildInputs = [ pkgs.pkg-config ];
              buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.alsa-lib ];
            };
            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          in
          {
            podcast-tui-source = source;

            podcast-tui-clippy = craneLib.cargoClippy (commonArgs // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- -D warnings";
            });

            podcast-tui-fmt = craneLib.cargoFmt {
              inherit src;
            };
          }
        );
      });
}
