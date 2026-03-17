{
  description = "Podcast TUI — a cross-platform terminal podcast manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;

        # Filter source to only Rust/Cargo-relevant files.
        # If build.rs or src/ ever needs non-Rust assets, replace with a custom filter.
        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; lib.optionals stdenv.isLinux [
            alsa-lib
          ];
        };

        # Build only dependencies (for caching — deps change less often than source)
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the full package
        podcast-tui = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;

          meta = with pkgs.lib; {
            description = "A cross-platform terminal user interface for podcast management";
            homepage = "https://github.com/lqdev/podcast-tui";
            license = licenses.mit;
            maintainers = [ ];
            platforms = platforms.linux;
            mainProgram = "podcast-tui";
          };
        });

      in
      {
        packages.default = podcast-tui;

        apps.default = flake-utils.lib.mkApp {
          drv = podcast-tui;
        };

        devShells.default = craneLib.devShell {
          inputsFrom = [ podcast-tui ];

          packages = with pkgs; [
            rust-analyzer
            cargo-watch
          ];
        };

        checks = {
          inherit podcast-tui;

          podcast-tui-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          });

          podcast-tui-fmt = craneLib.cargoFmt {
            inherit src;
          };
        };
      });
}
