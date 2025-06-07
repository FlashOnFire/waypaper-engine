{
  self,
  lib,
  rustPlatform,
  mold-wrapped,
}: let
  cargoRoot = "waypaper_engine_cli";
  cargoToml = lib.importTOML ../../${cargoRoot}/Cargo.toml;
  rev = self.shortRev or self.dirtyShortRev or "dirty";
in
  rustPlatform.buildRustPackage {
    pname = cargoToml.package.name;
    version = "${cargoToml.package.version}+git.${rev}";

    src = lib.cleanSource ../..;

    inherit cargoRoot;
    buildAndTestSubdir = cargoRoot;

    cargoLock.lockFile = ../../Cargo.lock;

    nativeBuildInputs = [
      mold-wrapped
    ];

    env.RUSTFLAGS = "-C link-arg=-fuse-ld=mold";

    postPatch = ''
      ln -s ${../../Cargo.lock} ${cargoRoot}/Cargo.lock
    '';

    doCheck = false;

    meta = {
      mainProgram = cargoToml.package.name;
      platforms = lib.platforms.linux;
    };
  }
