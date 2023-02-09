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
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" "x86_64-windows" ] (system:
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
      in
      with pkgs;
      {
        devShells.default = mkShell {
          buildInputs = [
            rust rust-analyzer
            rnix-lsp
            nixpkgs-fmt
            openssl
            pkg-config
          ];

          shellHook = ''
          '';
        };
      }
    );
}
