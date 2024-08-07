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
      in {
        packages = {
          default = self'.packages.waypaper-engine;
          waypaper-engine = pkgs.callPackage ./packaging/nix {};
        };

        devShells.default = with pkgs;
          mkShell rec {
            nativeBuildInputs = [pkg-config];

            buildInputs = [
              (rust-bin.stable.latest.default.override {
                extensions = ["rust-analyzer" "rust-src"];
              })

              mpv
              libGL
              libxkbcommon
              wayland
              webkitgtk_4_1
            ];

            LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
          };

        formatter = pkgs.alejandra;
      };
    };
}
