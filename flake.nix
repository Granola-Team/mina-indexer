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
        ];

        buildDependencies = with pkgs;
          [
            pkg-config
          ]
          ++ lib.optionals stdenv.isDarwin [darwin.apple_sdk.frameworks.Security]
          ++ runtimeDependencies;

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
            };

            default = mina-indexer;
          };

          devShells.default = mkShell {
            buildInputs = developmentDependencies;
            shellHook = ''
            '';
          };
        }
    );
}
