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

      # Define common libraries needed for both runtime and development
      commonLibs = with pkgs; [
        openssl
        zstd
        libffi
        gmp
        jemalloc
      ];

      runtimeDependencies = commonLibs;

      frameworks = pkgs.darwin.apple_sdk.frameworks;

      buildDependencies = with pkgs;
        [
          cargo-nextest
          clang
          libclang.lib
          pkg-config
          rustPlatform.bindgenHook
        ]
        ++ runtimeDependencies
        ++ lib.optionals stdenv.isDarwin [
          frameworks.Security
          frameworks.CoreServices
          zld
        ]
        ++ lib.optionals (!stdenv.isDarwin) [
          mold-wrapped # Linux only - https://github.com/rui314/mold#mold-a-modern-linker
        ];

      # used to ensure rustfmt is nightly version to support unstable features
      nightlyToolchain =
        pkgs.rust-bin.selectLatestNightlyWith (toolchain:
          toolchain.minimal.override {extensions = ["rustfmt"];});

      developmentDependencies = with pkgs;
        [
          alejandra
          biome
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
          mdformat
        ]
        ++ buildDependencies;

      # Platform-specific environment setup
      commonEnv = with pkgs.lib; let
        linuxEnv = {
          NIX_LD = "/run/current-system/sw/share/nix-ld/lib/ld.so";
          NIX_LD_LIBRARY_PATH = "/run/current-system/sw/share/nix-ld/lib";
          LD_LIBRARY_PATH = "${makeLibraryPath commonLibs}:/run/current-system/sw/share/nix-ld/lib";
        };
        darwinEnv = {
          LIBRARY_PATH = "${makeLibraryPath commonLibs}";
        };
      in
        if pkgs.stdenv.isDarwin
        then darwinEnv
        else linuxEnv;

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

            # This is equivalent to `git rev-parse --short=8 HEAD`
            gitCommitHash = builtins.substring 0 8 (self.rev or "dev");

            postPatch = ''ln -s "${./rust/Cargo.lock}" Cargo.lock'';
            preBuild = ''
              export GIT_COMMIT_HASH=${gitCommitHash}
              cd rust
            '';
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
              paths = with pkgs; [mina-indexer bash self] ++ commonLibs;
              name = "mina-indexer-root";
              pathsToLink = ["/bin" "/share" "/lib"];
            };
            config = {
              Cmd = ["${pkgs.lib.getExe mina-indexer}"];
              Env = lib.mapAttrsToList (name: value: "${name}=${value}") commonEnv;
            };
          };
        };

        devShells.default = mkShell {
          env =
            {
              LIBCLANG_PATH = "${libclang.lib}/lib";
            }
            // commonEnv;
          buildInputs = developmentDependencies;
          shellHook = lib.optionalString (!stdenv.isDarwin) ''
            export TMPDIR=/var/tmp

            # Create wrapper script for mina_txn_hasher.exe
            mkdir -p .local/bin
            cat > .local/bin/mina_txn_hasher.exe <<EOF
            #!/bin/sh
            exec ${pkgs.glibc}/lib/ld-linux-x86-64.so.2 ${toString ./.}/ops/mina/mina_txn_hasher.exe "\$@"
            EOF
            chmod +x .local/bin/mina_txn_hasher.exe
            export PATH="$PWD/.local/bin:$PATH"
          '';
        };
      });
}
