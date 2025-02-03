let 
  nixpkgs =
    import (builtins.fetchTarball {
      # Descriptive name to make the store path easier to identify
      name = "nixos-for-granola";
      url = "https://github.com/nixos/nixpkgs/archive/59e618d90c065f55ae48446f307e8c09565d5ab0.tar.gz";
      # Hash obtained using `nix-prefetch-url --unpack <url>`
      sha256 = "sha256-B/7Y1v4y+msFFBW1JAdFjNvVthvNdJKiN6EGRPnqfno=";
    }) {};

  runtimeDeps =
    with nixpkgs; [
      openssl
      zstd
      libffi
      gmp
      jemalloc
    ];

  buildDeps =
    with nixpkgs; [
      cargo-nextest
      clang
      libclang.lib
      pkg-config
      rustPlatform.bindgenHook
      mold-wrapped # Linux only - https://github.com/rui314/mold#mold-a-modern-linker
    ]
    ++ runtimeDeps;

  customBuildRustCrateForPkgs =
    pkgs: pkgs.buildRustCrate.override {
      defaultCrateOverrides = pkgs.defaultCrateOverrides // {
        libspeedb-sys = attrs: {
          buildInputs = buildDeps;
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        };
        mina-indexer = attrs: {
          preBuild = ''
            export GIT_COMMIT_HASH="$(
              find . -type f -print0 |
              sort -z |
              xargs -0 sha256sum |
              sha256sum |
              head -c 8
            )"
          '';
          checkPhase = ''
            set -ex
            cargo clippy --all-targets --all-features -- -D warnings
            cargo nextest run --release
          '';
          # preInstall = "mkdir -p $out/var/log/mina-indexer";
          # LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        };
      };
    };

  generatedBuild = nixpkgs.callPackage ./Cargo.nix {
    buildRustCrateForPkgs = customBuildRustCrateForPkgs;
  };
in
  generatedBuild.rootCrate.build.override {
    runTests = true;
  }
