{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/51d906d2341c9e866e48c2efcaac0f2d70bfd43e.tar.gz") {}
}:

pkgs.mkShell {
  LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_16.libclang.lib ];
  buildInputs = [
    pkgs.cargo
    pkgs.cargo-nextest
    pkgs.cargo-machete
    pkgs.clang
    pkgs.clippy
    pkgs.google-cloud-sdk  # For 'gsutil' in testing.
    pkgs.just
    pkgs.llvmPackages_16.bintools
  ];
  shellHook = ''
    export TMPDIR='/var/tmp'
  '';
}
