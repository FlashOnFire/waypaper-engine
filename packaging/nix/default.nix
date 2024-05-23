{
  lib,
  rustPlatform,
  pkg-config,
  wayland,
  libGL,
  libxkbcommon,
  mpv,
}:
rustPlatform.buildRustPackage {
  pname = "waypaper-engine";
  version = "0.0";
  src = lib.cleanSource ../..;

  cargoLock.lockFile = ../../Cargo.lock;

  buildInputs = [
    wayland
    libGL
    libxkbcommon
    mpv
  ];

  nativeBuildInputs = [
    pkg-config
  ];
}
