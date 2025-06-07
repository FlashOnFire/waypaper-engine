{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-parts,
    ...
  } @ inputs:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux" "aarch64-linux"];

      perSystem = {
        self',
        pkgs,
        system,
        ...
      }: {
        packages = {
          daemon = pkgs.callPackage ./packaging/nix/daemon.nix {inherit self;};
          ui = pkgs.callPackage ./packaging/nix/ui.nix {inherit self;};
          cli = pkgs.callPackage ./packaging/nix/cli.nix {inherit self;};
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = with self'.packages; [daemon ui];
          packages = with pkgs; [clippy rustfmt];

          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        };

        formatter = pkgs.alejandra;
      };
    };
}
