# Home Manager module for podcast-tui.
#
# Usage in your home-manager configuration:
#
#   { inputs, ... }: {
#     imports = [ inputs.podcast-tui.homeManagerModules.default ];
#     programs.podcast-tui.enable = true;
#   }
#
# This module exposes a minimal `enable` + `package` API. We intentionally do
# not surface declarative subscription/config management here — podcast-tui's
# config and subscription state live in `~/.local/share/podcast-tui` and are
# managed interactively. If declarative config is added later, additional
# options can be threaded in without breaking the existing API.
self: { config, lib, pkgs, ... }:

let
  cfg = config.programs.podcast-tui;
in
{
  options.programs.podcast-tui = {
    enable = lib.mkEnableOption "podcast-tui, a terminal user interface for podcast management";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.system}.default;
      defaultText = lib.literalExpression "podcast-tui.packages.\${pkgs.system}.default";
      description = ''
        The podcast-tui package to install. Defaults to the flake's
        `packages.default`, which transparently selects a prebuilt binary on
        supported platforms (x86_64-linux, aarch64-linux when published) and
        falls back to a source build elsewhere.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];
  };
}
