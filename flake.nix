{
  inputs = {
    nixpkgs.url = "nixpkgs";

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-parts,
    rust-overlay,
    ...
  } @ inputs:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];

      perSystem = {
        self',
        lib,
        system,
        ...
      }: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import rust-overlay)];
        };

        rustVersion = pkgs.rust-bin.nightly.latest.default;
        buildInputs = with pkgs; [
          llvmPackages.clang
          libGL
          libxkbcommon
          wayland
          webkitgtk_4_1
          ffmpeg-full
          libclang
        ];
        nativeBuildInputs = with pkgs; [pkg-config];
      in {
        packages = {
          default = self'.packages.waypaper-engine;
          waypaper-engine = pkgs.callPackage ./packaging/nix {
            inherit rustVersion buildInputs nativeBuildInputs;
          };
        };

        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs;

          buildInputs =
            [
              (rustVersion.override {
                extensions = ["rust-analyzer" "rust-src" "clippy"];
              })
            ]
            ++ buildInputs;

          LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
        };

        formatter = pkgs.alejandra;
      };
    };
}
