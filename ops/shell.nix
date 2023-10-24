{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/2281c1ca636ae9164b00e33f01190a01895282fc.tar.gz") {}
}:

pkgs.mkShell {
  buildInputs = [
    pkgs.cargo
    # pkgs.clang
    pkgs.just        # For running the build tool.
    # pkgs.libiconv    # Required for compiling Rust tools.
    # pkgs.llvmPackages.bintools
    # pkgs.openssl     # Required for compiling.
    # pkgs.pkg-config  # Required for compiling.
    # pkgs.postgresql  # Required for compiling against libpq, and for pg_isready.
    # pkgs.rustup
  ];
  shellHook = ''
    # export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
    # export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
    # export TMPDIR=/var/tmp
  '';
}
