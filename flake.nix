{
  description = "development environment for mina-indexer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, flake-compat, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rust = pkgs.rust-bin.fromRustupToolchainFile ./toolchain.toml;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };

        dependencies = with pkgs; [
          pkg-config
          openssl
        ];

        devDependencies = with pkgs; [
          rust
          rust-analyzer
          rnix-lsp
          nixpkgs-fmt
        ] ++ dependencies;
      in
      with pkgs;
      {
        packages = flake-utils.lib.flattenTree rec {
          mina-indexer = rustPlatform.buildRustPackage rec {
            pname = "mina-indexer";
            version = "0.1.1";

            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = dependencies;
            buildInputs = dependencies;
          };

          default = mina-indexer;
        };

        devShells.default = mkShell {
          buildInputs = devDependencies;
          shellHook = ''
          '';
        };
      }
    );
}
