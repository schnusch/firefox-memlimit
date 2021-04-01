{ config, lib, pkgs, ... }:

with lib;

let

  cfg = config.programs.firefox-memlimit;
  package = pkgs.callPackage ./package.nix {};

in

{
  options = {
    programs.firefox-memlimit.enable = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Whether to <literal>enable</literal> firefox-memlimit.
      '';
    };
  };

  config = mkIf cfg.enable {
    security.wrappers.firefox-memlimit.source = "${package.out}/bin/firefox-memlimit";
  };
}
