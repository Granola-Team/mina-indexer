{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    flake-compat,
    ...
    }:
    flake-utils.lib.eachSystem ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"] (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };

        runtimeDependencies = with pkgs; [
          openssl
        ];

        frameworks = pkgs.darwin.apple_sdk.frameworks;

        buildDependencies = with pkgs; [
            libclang.lib
            clang
            pkg-config
            rustPlatform.bindgenHook]
          ++ runtimeDependencies
          ++ lib.optionals stdenv.isDarwin [
            frameworks.Security
            frameworks.CoreServices
          ];

        developmentDependencies = with pkgs;
          [
            rust
            cargo-nextest
            cargo-audit
            cargo-machete
            google-cloud-sdk
            just
            jq       # Used in testing.
            git      # Needed but not declared by Nix's 'stdenv' build.
          ]
          ++ buildDependencies;

        cargo-toml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      in
        with pkgs; {
          packages = flake-utils.lib.flattenTree rec {
            mina-indexer = rustPlatform.buildRustPackage rec {
              pname = cargo-toml.package.name;
              version = cargo-toml.package.version;

              src = ./.;
              cargoLock = {
                lockFile = ./Cargo.lock;
              };

              nativeBuildInputs = buildDependencies;
              buildInputs = runtimeDependencies;

              env = { LIBCLANG_PATH = "${libclang.lib}/lib"; }
              // (lib.optionalAttrs (stdenv.cc.isClang && stdenv.isDarwin) { NIX_LDFLAGS = "-l${stdenv.cc.libcxx.cxxabi.libName}"; });

              doCheck = false;
            };

            default = mina-indexer;
          };

          devShells.default = mkShell {
            NIX_LDFLAGS="-l${stdenv.cc.libcxx.cxxabi.libName}";
            buildInputs = developmentDependencies;
            shellHook = ''
              git submodule update --init --recursive
              export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
              export TMPDIR=/var/tmp
            '';
          };
        }
    );
}
