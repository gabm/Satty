{
  description = "A basic Rust devshell for NixOS users developing gtk/libadwaita apps";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

      in
      with pkgs;
      {
        devShells.default = mkShell {
          buildInputs = [
            pkg-config
            libGL
            libepoxy
            gtk4
            wrapGAppsHook4 # this is needed for relm4-icons to properly load after gtk::init()
            libadwaita

            (rust-bin.stable.latest.default.override
              {
                extensions = [ "rust-src" ];
              })
          ];

          shellHook = ''
            export GSETTINGS_SCHEMA_DIR=${glib.getSchemaPath gtk4}
          '';
        };
      }
    );
}
