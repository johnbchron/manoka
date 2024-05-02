{
  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.2311.557010.tar.gz";
    rust-overlay = {
      url = "https://flakehub.com/f/oxalica/rust-overlay/0.1.1330.tar.gz";
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
          toolchain mold clang bacon
          pkg-config
        ];
        buildInputs = if !pkgs.stdenv.isDarwin then (with pkgs; [
          udev alsa-lib vulkan-loader
          xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr # To use the x11 feature
          vulkan-validation-layers
          libxkbcommon wayland
          wgsl-analyzer.packages."${system}".default
        ]) else (with pkgs; [
          darwin.CF darwin.apple_sdk.frameworks.Cocoa darwin.apple_sdk.frameworks.CoreServices
        ]);

      in {
        devShell = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;

          shellHook = ''
            export LD_LIBRARY_PATH="$GLOBAL_LIBGL:$LD_LIBRARY_PATH"
          '';
        };
      }
  );
}
