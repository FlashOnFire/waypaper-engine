{
  lib,
  pkgs,
  rustVersion,
  buildInputs,
  nativeBuildInputs,
}: let
  rustPlatform = pkgs.makeRustPlatform {
    cargo = rustVersion;
    rustc = rustVersion;
  };
in
  rustPlatform.buildRustPackage {
    pname = "waypaper-engine";
    version = "0.1.0";
    src = lib.cleanSource ../..;

    cargoLock.lockFile = ../../Cargo.lock;

    inherit buildInputs;

    nativeBuildInputs =
      nativeBuildInputs ++ [rustPlatform.bindgenHook];
  }
