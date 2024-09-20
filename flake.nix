{
  inputs = {
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixpkgs.url = "github:NixOS/nixpkgs?ref=931494da4b60fb26719e231d6de4b2c96167a1ce";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachSystem [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ] (system: let
      overlays = [(import rust-overlay)];

      pkgs = import nixpkgs {inherit system overlays;};

      rust = pkgs.rust-bin.fromRustupToolchainFile ./rust/rust-toolchain.toml;

      rustPlatform = pkgs.makeRustPlatform {
        cargo = rust;
        rustc = rust;
      };

      runtimeDependencies = with pkgs; [openssl zstd];

      frameworks = pkgs.darwin.apple_sdk.frameworks;

      buildDependencies = with pkgs;
        [
          cargo-nextest
          clang
          libclang.lib
          mold-wrapped # https://github.com/rui314/mold#mold-a-modern-linker
          pkg-config
          rustPlatform.bindgenHook
        ]
        ++ runtimeDependencies
        ++ lib.optionals stdenv.isDarwin [
          frameworks.Security
          frameworks.CoreServices
          zld
        ];

      # used to ensure rustfmt is nightly version to support unstable features
      nightlyToolchain =
        pkgs.rust-bin.selectLatestNightlyWith (toolchain:
          toolchain.minimal.override {extensions = ["rustfmt"];});

      developmentDependencies = with pkgs;
        [
          alejandra
          cargo-audit
          cargo-machete
          curl
          check-jsonschema
          git # Needed but not declared by Nix's 'stdenv' build.
          google-cloud-sdk # Required only to use O1's bucket.
          hurl
          jq
          just
          nightlyToolchain.passthru.availableComponents.rustfmt
          nix-output-monitor # Use 'nom' in place of 'nix' to use this.
          rclone
          ruby
          rubyPackages.standard
          rubyPackages.rspec
          rust
          shellcheck
          shfmt
        ]
        ++ buildDependencies;

      cargo-toml = builtins.fromTOML (builtins.readFile ./rust/Cargo.toml);
    in
      with pkgs; {
        packages = flake-utils.lib.flattenTree rec {
          mina-indexer = rustPlatform.buildRustPackage rec {
            meta = with lib; {
              description = ''
                The Mina Indexer is a re-imagined version of the software collectively called the "Mina archive node."
              '';
              homepage = "https://github.com/Granola-Team/mina-indexer";
              license = licenses.asl20;
              mainProgram = "mina-indexer";
              platforms = platforms.all;
              maintainers = [];
            };

            pname = cargo-toml.package.name;

            version = cargo-toml.package.version;

            src = lib.cleanSourceWith {
              src = lib.cleanSource ./.;
              filter = path: type:
                (path != ".direnv")
                && (path != "rust/target")
                && (path != "ops")
                && (path != "Justfile")
                && (path != "tests");
            };

            cargoLock = {lockFile = ./rust/Cargo.lock;};

            nativeBuildInputs = buildDependencies;

            buildInputs = runtimeDependencies;

            # env = { LIBCLANG_PATH = "${libclang.lib}/lib"; };

            # This is equivalent to `git rev-parse --short=8 HEAD`
            gitCommitHash = builtins.substring 0 8 (self.rev or "dev");

            postPatch = ''ln -s "${./rust/Cargo.lock}" Cargo.lock'';
            preBuild = ''
              export GIT_COMMIT_HASH=${gitCommitHash}
              cd rust
            '';
            # TODO:
            # doCheck = true;
            checkPhase = ''
              set -ex
              cargo clippy --all-targets --all-features -- -D warnings
              cargo nextest run --release
            '';
            preInstall = "mkdir -p $out/var/log/mina-indexer";
          };

          default = mina-indexer;

          dockerImage = pkgs.dockerTools.buildImage {
            name = "mina-indexer";
            created = "now";
            tag = builtins.substring 0 8 (self.rev or "dev");
            copyToRoot = pkgs.buildEnv {
              paths = with pkgs; [mina-indexer openssl zstd bash self];
              name = "mina-indexer-root";
              pathsToLink = ["/bin" "/share"];
            };
            config.Cmd = ["${pkgs.lib.getExe mina-indexer}"];
          };
        };

        devShells.default = mkShell {
          env = {LIBCLANG_PATH = "${libclang.lib}/lib";};
          buildInputs = developmentDependencies;
          shellHook = "export TMPDIR=/var/tmp";
        };
      });
}
