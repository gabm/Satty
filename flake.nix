{
  description = "A basic Rust devshell for NixOS users developing gtk/libadwaita apps";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    ...
  }: let
    systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    forEachSystem = nixpkgs.lib.genAttrs systems;
  in {
    devShells = forEachSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {inherit system overlays;};
      in rec {
        default = satty;
        satty = pkgs.mkShell {
          buildInputs = with pkgs; [
            pkg-config
            libGL
            libepoxy
            gtk4
            wrapGAppsHook4 # this is needed for relm4-icons to properly load after gtk::init()
            libadwaita
            fontconfig

            (rust-bin.stable.latest.default.override {
              extensions = ["rust-src"];
            })
          ];

          shellHook = with pkgs; ''
            export GSETTINGS_SCHEMA_DIR=${glib.getSchemaPath gtk4}
          '';
        };
      }
    );
  };
}
