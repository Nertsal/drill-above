{
  description = "Drill dev";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        libDeps = with pkgs; [
            alsa-lib
            cmake
            fontconfig
            udev
            libGL
            xorg.libxcb
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            libxkbcommon
            wayland
        ];
        libPath = pkgs.lib.makeLibraryPath libDeps;
      in
      with pkgs;
      {
        devShells.default = mkShell {
          buildInputs = libDeps ++ [
            gcc
            openssl
            pkg-config
            # rust-bin.stable.latest.default
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" ];
              targets = [ "wasm32-unknown-unknown" ];
            })
          ];
          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${libPath}"
          '';
        };
      }
    );
}

