with import <nixpkgs> {}; let
  deps = [
    jemalloc
    openssl
    libffi
    gmp
    gcc.cc.lib
  ];
in
  stdenv.mkDerivation {
    name = "mina_txn_hasher";
    src = ./.;

    dontConfigure = true;
    dontBuild = true;

    nativeBuildInputs = [autoPatchelfHook];
    buildInputs = deps;

    installPhase = ''
      mkdir -p $out/bin
      install -Dm755 mina_txn_hasher.exe $out/bin/mina_txn_hasher.exe
    '';
  }
