{
  self,
  lib,
  rustPlatform,
  ffmpeg-full,
  libGL,
  libxkbcommon,
  mold-wrapped,
  pkg-config,
  wayland,
  makeWrapper,
}:
let
  cargoRoot = "waypaper_engine_daemon";
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
    pkg-config
    rustPlatform.bindgenHook
    makeWrapper
  ];

  buildInputs = [
    ffmpeg-full
    wayland
    libGL
    libxkbcommon
  ];

  env.RUSTFLAGS = "-C link-arg=-fuse-ld=mold";

  postPatch = ''
    ln -s ${../../Cargo.lock} ${cargoRoot}/Cargo.lock
  '';

  doCheck = false;

  postInstall = ''
    wrapProgram $out/bin/${cargoToml.package.name} \
      --set LD_LIBRARY_PATH ${lib.makeLibraryPath [ libGL ]}
  '';

  meta = {
    mainProgram = cargoToml.package.name;
    platforms = lib.platforms.linux;
  };
}
