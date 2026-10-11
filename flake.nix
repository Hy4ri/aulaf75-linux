{
  description = "aulaf75-linux: Native Linux GUI & HID driver for AULA F75 keyboard in Rust (GPUI)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});
    in
    {
      packages = forAll (pkgs:
        let
          lib = pkgs.lib;
          runtimeLibs = with pkgs; [
            vulkan-loader
            wayland
            libxkbcommon
            libx11
            libxcb
            fontconfig
            freetype
            systemd # for libudev
          ];
        in
        rec {
          aulaf75-linux = pkgs.rustPlatform.buildRustPackage {
            pname = "aulaf75-linux";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = with pkgs; [
              pkg-config
              cmake
              makeWrapper
            ];

            buildInputs = with pkgs; [
              fontconfig
              freetype
              libxkbcommon
              wayland
              libxcb
              libx11
              vulkan-loader
              udev
            ];

            postFixup = ''
              wrapProgram $out/bin/aulaf75-linux \
                --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs}
            '';
          };
          default = aulaf75-linux;
        });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
            pkg-config
            udev
          ];
          buildInputs = with pkgs; [
            fontconfig
            freetype
            libxkbcommon
            wayland
            libxcb
            libx11
            vulkan-loader
            udev
          ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
            vulkan-loader
            wayland
            libxkbcommon
            libx11
            libxcb
            fontconfig
            freetype
            udev
            systemd
          ]);
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });
    };
}
