{
  lib,
  # rustPlatform,
  makeRustPlatform,
  rust-bin,
  pkg-config,
  wayland,
  libGL,
  libxkbcommon,
  mpv,
  webkitgtk_4_1,
}: let
  rustPlatform = makeRustPlatform {
    cargo = rust-bin.stable.latest.default;
    rustc = rust-bin.stable.latest.default;
  };
in
  rustPlatform.buildRustPackage {
    pname = "waypaper-engine";
    version = "0.1.0";
    src = lib.cleanSource ../..;

    cargoLock.lockFile = ../../Cargo.lock;

    buildInputs = [
      wayland
      libGL
      libxkbcommon
      mpv
      webkitgtk_4_1
    ];

    nativeBuildInputs = [
      pkg-config
    ];
  }
