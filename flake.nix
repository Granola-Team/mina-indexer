{
  inputs = {
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];

        pkgs = import nixpkgs { inherit system overlays; };

        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust/rust-toolchain.toml;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };

        mina_txn_hasher = pkgs.callPackage ./ops/mina/mina_txn_hasher.nix { };

        frameworks = pkgs.darwin.apple_sdk.frameworks;

        buildDependencies =
          with pkgs;
          [ rustPlatform.bindgenHook ]
          ++ lib.optionals stdenv.isDarwin [
            frameworks.Security
            frameworks.CoreServices
            lld_20 # A faster linker
          ]
          ++ lib.optionals (!stdenv.isDarwin) [
            mold-wrapped # Linux only - https://github.com/rui314/mold#mold-a-modern-linker
          ];

        # used to ensure rustfmt is nightly version to support unstable features
        nightlyToolchain = pkgs.rust-bin.selectLatestNightlyWith (
          toolchain: toolchain.minimal.override { extensions = [ "rustfmt" ]; }
        );

        developmentDependencies =
          with pkgs;
          [
            biome
            cargo-audit
            cargo-machete
            cargo-nextest
            clang # For clang in shell
            curl
            check-jsonschema
            git # Needed but not declared by Nix's 'stdenv' build.
            hurl
            jq
            nightlyToolchain.passthru.availableComponents.rustfmt
            nix-output-monitor # Use 'nom' in place of 'nix' to use this.
            nixfmt-rfc-style # For formatting Nix code.
            openssh # Needed by 'git' but not declared.
            rclone
            ruby
            rubyPackages.standard
            rubyPackages.rspec
            rust
            shellcheck
            shfmt
            mdformat
            samply # rust profiling
          ]
          ++ buildDependencies;

        cargo-toml = builtins.fromTOML (builtins.readFile ./rust/Cargo.toml);
      in
      with pkgs;
      {
        packages = flake-utils.lib.flattenTree rec {
          inherit mina_txn_hasher;

          mina-indexer = rustPlatform.buildRustPackage rec {
            meta = with lib; {
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
              filter =
                path: type:
                (path != ".direnv")
                && ((path == "rust/.cargo") || (path == "rust/.cargo/config.toml") || (dirOf path != "rust/.cargo"))
                && (path != "result")
                && (path != ".build")
                && (path != "rust/target")
                && (path != "ops")
                && (path != "Justfile")
                && (path != "Rakefile")
                && (path != "tests");
            };

            cargoLock = {
              lockFile = ./rust/Cargo.lock;
            };

            nativeBuildInputs = buildDependencies;

            # This is equivalent to `git rev-parse --short=8 HEAD`
            gitCommitHash = builtins.substring 0 8 (self.rev or (abort "Nix build requires a clean Git repo."));

            postPatch = ''ln -s "${./rust/Cargo.lock}" Cargo.lock'';
            preBuild = ''
              export GIT_COMMIT_HASH=${gitCommitHash}
              cd rust
            '';
            doCheck = false;
            preInstall = "mkdir -p $out/var/lib/mina-indexer";
          };

          default = mina-indexer;

          dockerImage = pkgs.dockerTools.buildImage {
            name = "mina-indexer";
            created = "now";
            tag = builtins.substring 0 8 (self.rev or "dev");
            copyToRoot = pkgs.buildEnv {
              paths = with pkgs; [
                mina-indexer
                bash
                self
              ];
              name = "mina-indexer-root";
              pathsToLink = [
                "/bin"
                "/share"
                "/lib"
              ];
            };
            config = {
              Cmd = [ "${pkgs.lib.getExe mina-indexer}" ];
            };
          };
        };

        devShells.default = mkShell {
          buildInputs = developmentDependencies ++ lib.optional (!stdenv.isDarwin) mina_txn_hasher; # for backwards compatibility
        };
      }
    );
}
