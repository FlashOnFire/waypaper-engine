{
  rustPlatform,
  lib,
  wayland,
  pkg-config,
  glew-egl,
  libxkbcommon,
  mpv,
}:
rustPlatform.buildRustPackage {
  pname = "waypaper-engine";
  version = "0.0";
  cargoLock.lockFile = ../../Cargo.lock;
  src = lib.cleanSource ../..;

  buildInputs = [
    wayland
    glew-egl
    libxkbcommon
    mpv
  ];

  nativeBuildInputs = [
    pkg-config
  ];
}
