# NixOS module for podcast-tui.
#
# Usage in your NixOS configuration:
#
#   { inputs, ... }: {
#     imports = [ inputs.podcast-tui.nixosModules.default ];
#     programs.podcast-tui.enable = true;
#   }
#
# Installs podcast-tui system-wide. For per-user installation, prefer the
# Home Manager module (`homeManagerModules.default`).
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
      description = "The podcast-tui package to install system-wide.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
  };
}
