{
  description = "development environment and build system for mina-indexer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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
    flake-utils.lib.eachSystem ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" "x86_64-windows"] (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rust = pkgs.rust-bin.fromRustupToolchainFile ./toolchain.toml;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };

        runtimeDependencies = with pkgs; [
          openssl
          zstd
        ];

        buildDependencies = with pkgs;
          [
            llvmPackages.libclang
            clang
            pkg-config
          ]
          ++ lib.optionals stdenv.isDarwin [darwin.apple_sdk.frameworks.Security];

        developmentDependencies = with pkgs;
          [
            rust
            rust-analyzer
            rnix-lsp
            alejandra
            pre-commit
            cargo-nextest
          ]
          ++ buildDependencies;

        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        BINDGEN_EXTRA_CLANG_ARGS =
          if pkgs.stdenv.isDarwin
          then "-isystem ${pkgs.stdenv.cc.cc}/lib/clang/${pkgs.lib.getVersion pkgs.stdenv.cc.cc}/include"
          else "-isystem ${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.lib.getVersion pkgs.clang}/include";

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

              preBuild = ''
                export LIBCLANG_PATH="${LIBCLANG_PATH}"
                export BINDGEN_EXTRA_CLANG_ARGS="${BINDGEN_EXTRA_CLANG_ARGS}"

              '';
              doCheck = false;
            };

            default = mina-indexer;
          };

          devShells.default = mkShell {
            buildInputs = developmentDependencies;
            shellHook = ''
              git submodule update --init --recursive --remote
              export PATH=.$out/bin:$PATH
              export LIBCLANG_PATH="${LIBCLANG_PATH}"
              export BINDGEN_EXTRA_CLANG_ARGS="${BINDGEN_EXTRA_CLANG_ARGS}"
            '';
          };
        }
    );
}
