{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-parts,
      rust-overlay,
      ...
    }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      perSystem =
        {
          self',
          pkgs,
          system,
          ...
        }:
        {
          _module.args.pkgs = import nixpkgs {
            inherit system;
            config = { };
            overlays = [ rust-overlay.overlays.default ];
          };

          packages =
            let
              rust = pkgs.rust-bin.stable.latest.default.override {
                extensions = [ "rust-src" ];
              };

              rustPlatform = pkgs.makeRustPlatform {
                rustc = rust;
                cargo = rust;
              };
            in
            {
              daemon = pkgs.callPackage ./packaging/nix/daemon.nix { inherit self rustPlatform; };
              ui = pkgs.callPackage ./packaging/nix/ui.nix { inherit self rustPlatform; };
              cli = pkgs.callPackage ./packaging/nix/cli.nix { inherit self rustPlatform; };
            };

          devShells.default = pkgs.mkShell {
            inputsFrom = with self'.packages; [
              daemon
              ui
            ];
            packages = with pkgs; [
              clippy
              rustfmt
            ];

            RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          };

          formatter = pkgs.nixfmt-tree;
        };
    };
}
