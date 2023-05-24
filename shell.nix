with import ./nix/pkgs.nix {};
let merged-openssl = pkgs.symlinkJoin { name = "merged-openssl"; paths = [ pkgs.openssl.out pkgs.openssl.dev ]; };
in stdenv.mkDerivation rec {
  name = "rust-env";
  env = buildEnv { name = name; paths = buildInputs; };
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang}/lib";
  OPENSSL_DIR = "${merged-openssl}";

  buildInputs = [
    rustup
    clang
    llvm
    llvmPackages.libclang
    openssl
    cacert
    niv
    pkg-config
    alsa-lib
    libudev-zero
    xorg.libX11
    xorg.libXi
    xorg.libXinerama
    xorg.libXext
    xorg.libXcursor
    xorg.libXrandr
    libGL
    SDL2
  ];

  APPEND_LIBRARY_PATH = lib.makeLibraryPath [
    libGL
    xorg.libX11
    xorg.libXi
    xorg.libXinerama
    xorg.libXext
    xorg.libXcursor
    xorg.libXrandr
  ];

  shellHook = ''
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:$APPEND_LIBRARY_PATH"
  '';
}
