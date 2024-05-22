{
  rustPlatform,
  lib,
  wayland,
  pkg-config,
  glew-egl,
  libxkbcommon,
  mpv,
  jxrlib,
  zlib,
  libwebp,
  libtiff,
  libpng,
  libraw,
  openjpeg,
  openexr,
  imath,
}:
rustPlatform.buildRustPackage {
  pname = "waypaper-engine";
  version = "0.0";
  cargoLock = {
    lockFile = ../../Cargo.lock;
    outputHashes = {
      "freeimage-sys-3.18.4" = "sha256-+S5G6cFAfoJUe9h5EgUwWFHZeBWZ5EpQJfljwX52Olk=";
    };
  };

  src = lib.cleanSource ../..;

  buildInputs = [
    wayland
    glew-egl
    libxkbcommon
    mpv
    jxrlib
    zlib
    libwebp
    libtiff
    libpng
    libraw
    openjpeg
    openexr
    imath
  ];

  nativeBuildInputs = [
    pkg-config
  ];
}
