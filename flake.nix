{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    wgsl-analyzer.url = "github:wgsl-analyzer/wgsl-analyzer";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, wgsl-analyzer }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        toolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        });

        nativeBuildInputs = with pkgs; [
          toolchain pkg-config clang
          alsa-lib udev

          libxkbcommon wayland
          xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr
 
          vulkan-headers vulkan-loader
          vulkan-tools vulkan-tools-lunarg
          vulkan-extension-layer
          vulkan-validation-layers

          wgsl-analyzer.packages."${system}".default
        ];
        buildInputs = if !pkgs.stdenv.isDarwin then (with pkgs; [
        ]) else (with pkgs; [
          darwin.CF darwin.apple_sdk.frameworks.Cocoa darwin.apple_sdk.frameworks.CoreServices
        ]);

      in {
        devShell = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath nativeBuildInputs;
        };
      }
  );
}
