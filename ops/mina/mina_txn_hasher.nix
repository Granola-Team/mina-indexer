{
  lib,
  stdenv,
  autoPatchelfHook,
  jemalloc,
  openssl,
  libffi,
  gmp,
  gcc,
}:
stdenv.mkDerivation {
  pname = "mina_txn_hasher";
  version = "1.0.0";

  src = ./.;

  dontConfigure = true;
  dontBuild = true;

  nativeBuildInputs = [autoPatchelfHook];
  buildInputs = [
    jemalloc
    openssl
    libffi
    gmp
    gcc.cc.lib
  ];

  installPhase = ''
    mkdir -p $out/bin
    install -Dm755 mina_txn_hasher.exe $out/bin/mina_txn_hasher.exe
  '';

  meta = with lib; {
    platforms = platforms.linux;
  };
}
