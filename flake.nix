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
          zstd
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

        # used to ensure rustfmt is nightly version to support unstable features
        nightlyToolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain:
          toolchain.minimal.override {
            extensions = ["rustfmt"];
          }
        );

        developmentDependencies = with pkgs;
          [
            nightlyToolchain.passthru.availableComponents.rustfmt
            rust
            cargo-nextest
            cargo-audit
            cargo-machete
            google-cloud-sdk
            just
            jq       # Used in testing.
            git      # Needed but not declared by Nix's 'stdenv' build.
            curl
            check-jsonschema
          ]
          ++ buildDependencies;

        cargo-toml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      in
        with pkgs; {
          packages = flake-utils.lib.flattenTree rec {
            mina-indexer = rustPlatform.buildRustPackage rec {
              meta = with lib; {
                description = "The Mina Indexer is a Mina blockchain analytics tool";
                longDescription = ''
                  The Mina Indexer is a re-designed version of the software collectively called the "Mina archive node."
                '';
                homepage = "https://github.com/Granola-Team/mina-indexer";
                license = licenses.asl20;
                mainProgram = "mina-indexer";
                platforms = platforms.all;
              };
              pname = cargo-toml.package.name;
              version = cargo-toml.package.version;

              src = ./.;
              cargoLock = {
                lockFile = ./Cargo.lock;
              };

              nativeBuildInputs = buildDependencies;
              buildInputs = runtimeDependencies;

              env = { LIBCLANG_PATH = "${libclang.lib}/lib"; } //
                    (lib.optionalAttrs (stdenv.cc.isClang && stdenv.isDarwin) { NIX_LDFLAGS = "-l${stdenv.cc.libcxx.cxxabi.libName}"; });

              doCheck = false;
            };
            default = mina-indexer;
            dockerImage = pkgs.dockerTools.buildImage {
              name = "mina-indexer";
              created = "now";
              # This is equivalent to `git rev-parse --short HEAD`
              tag = builtins.substring 0 9 (self.rev or "dev");
              copyToRoot = pkgs.buildEnv {
                paths = with pkgs; [
                  mina-indexer
                  openssl
                  zstd
                  bash
                  self
                ];
                name = "idx-root";
                pathsToLink = [ "/bin" ];
              };
              config.Cmd = [ "${pkgs.lib.getExe mina-indexer}" ];
            };

          };

          devShells.default = mkShell {
            env = { LIBCLANG_PATH = "${libclang.lib}/lib"; } //
                  (lib.optionalAttrs (stdenv.cc.isClang && stdenv.isDarwin) { NIX_LDFLAGS = "-l${stdenv.cc.libcxx.cxxabi.libName}"; });

            buildInputs = developmentDependencies;
            shellHook = ''
              export TMPDIR=/var/tmp
            '';
          };
        }
    );
}
