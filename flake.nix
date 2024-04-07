{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachSystem [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ] (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust/rust-toolchain.toml;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };

        runtimeDependencies = with pkgs; [ openssl zstd ];

        frameworks = pkgs.darwin.apple_sdk.frameworks;

        buildDependencies = with pkgs;
          [
            cargo-machete
            cargo-nextest
            check-jsonschema
            clang
            curl
            jq
            libclang.lib
            pkg-config
            rustPlatform.bindgenHook
          ] ++ runtimeDependencies ++ lib.optionals stdenv.isDarwin [
            frameworks.Security
            frameworks.CoreServices
          ];

        # used to ensure rustfmt is nightly version to support unstable features
        nightlyToolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain:
          toolchain.minimal.override { extensions = [ "rustfmt" ]; });

        developmentDependencies = with pkgs;
          [
            cargo-audit
            git # Needed but not declared by Nix's 'stdenv' build.
            just
            nightlyToolchain.passthru.availableComponents.rustfmt
            nix-output-monitor # Use 'nom' in place of 'nix' to use this.
            rust
          ] ++ buildDependencies;

        cargo-toml = builtins.fromTOML (builtins.readFile ./rust/Cargo.toml);
      in with pkgs; {
        packages = flake-utils.lib.flattenTree rec {
          mina-indexer = rustPlatform.buildRustPackage rec {
            meta = with lib; {
              description = ''
                The Mina Indexer is a re-imagined version of the software collectively called the "Mina archive node."
              '';
              longDescription = ''
                The Mina Indexer (indexer) is a re-imagined version of the software collectively called the "Mina archive node." It uses precomputed blocks to reconstitute the historical state of the Mina blockchain. The redesign focuses on improving ease of use and accessibility for developers to interact with the Mina blockchain.
              '';
              homepage = "https://github.com/Granola-Team/mina-indexer";
              license = licenses.asl20;
              mainProgram = "mina-indexer";
              platforms = platforms.all;
              maintainers = [ ];
            };
            pname = cargo-toml.package.name;
            version = cargo-toml.package.version;

            src = lib.cleanSourceWith {
              src = lib.cleanSource ./.;
              filter = path: type:
                (path != ".direnv") && (path != "rust/target");
            };
            dataDir = ./data;

            cargoLock = { lockFile = ./rust/Cargo.lock; };

            nativeBuildInputs = buildDependencies;
            buildInputs = runtimeDependencies;

            env = { LIBCLANG_PATH = "${libclang.lib}/lib"; };

            postPatch = ''
              ln -s "${./rust/Cargo.lock}" Cargo.lock
              patchShebangs tests/regression
              patchShebangs tests/download_blocks
            '';
            preBuild = "cd rust";
            postBuild = ''
              set -ex
              mkdir -p $out/share/mina-indexer/data
              cp ${dataDir}/locked.csv $out/share/mina-indexer/data/locked.csv
              cp -r ${dataDir}/genesis_blocks/mainnet-1-3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ.json $out/share/mina-indexer/data
              cp -r ${dataDir}/genesis_ledgers/mainnet.json $out/share/mina-indexer/data
            '';
            doCheck = true;
            checkPhase = ''
              set -ex
              cargo fmt --all --check
              cargo clippy --all-targets --all-features -- -D warnings
              cargo machete Cargo.toml
              cargo nextest run --release
              ../tests/regression
            '';
            preInstall = ''
              mkdir -p $out/var/log/mina-indexer
              mkdir -p $out/var/lib/mina-indexer
            '';
          };
          default = mina-indexer;
          dockerImage = pkgs.dockerTools.buildImage {
            name = "mina-indexer";
            created = "now";
            # This is equivalent to `git rev-parse --short HEAD`
            tag = builtins.substring 0 9 (self.rev or "dev");
            copyToRoot = pkgs.buildEnv {
              paths = with pkgs; [ mina-indexer openssl zstd bash self ];
              name = "mina-indexer-root";
              pathsToLink = [ "/bin" "/share" ];
            };
            config.Cmd = [ "${pkgs.lib.getExe mina-indexer}" ];
            config.Env = [ "RELEASE=production" ];
          };
        };

        devShells.default = mkShell {
          env = { LIBCLANG_PATH = "${libclang.lib}/lib"; };
          buildInputs = developmentDependencies;
          shellHook = ''
            export TMPDIR=/var/tmp
          '';
        };
      });
}
