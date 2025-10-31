{
  self,
  lib,
  rustPlatform,
  mold-wrapped,
  pkg-config,
  pnpm_9,
  webkitgtk_4_1,
  wrapGAppsHook3,
}:
let
  projectRoot = "waypaper_engine_ui";
  cargoToml = lib.importTOML ../../${projectRoot}/src-tauri/Cargo.toml;
  rev = self.shortRev or self.dirtyShortRev or "dirty";
in
rustPlatform.buildRustPackage rec {
  pname = cargoToml.package.name;
  version = "${cargoToml.package.version}+git.${rev}";

  src = lib.cleanSource ../..;
  sourceRoot = "${src.name}/${projectRoot}";

  cargoRoot = "src-tauri";
  buildAndTestSubdir = cargoRoot;

  cargoLock.lockFile = ../../Cargo.lock;

  pnpmDeps = pnpm_9.fetchDeps {
    inherit
      pname
      version
      src
      sourceRoot
      ;
    fetcherVersion = 2;
    hash = "sha256-tFOPGIeLf1ECRG11tIfhKSlQjdX1OvBMbDhlPPB6ShQ=";
  };

  nativeBuildInputs = [
    mold-wrapped
    pkg-config
    pnpm_9.configHook
    rustPlatform.bindgenHook
    wrapGAppsHook3
  ];

  buildInputs = [
    webkitgtk_4_1
  ];

  env.RUSTFLAGS = "-C link-arg=-fuse-ld=mold";

  postPatch = ''
    ln -s ${../../Cargo.lock} ${cargoRoot}/Cargo.lock
  '';

  doCheck = false;

  meta = {
    mainProgram = pname;
    platforms = lib.platforms.linux;
  };
}
