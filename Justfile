# Justfile
#
# The command 'just' will give usage information.
# See https://github.com/casey/just for more.

# Ensure rustfmt works in all environments
# Nix environment has rustfmt nightly and won't work with +nightly
# Non-Nix environment needs nightly toolchain installed and requires +nightly
is_rustfmt_nightly := `cd rust && rustfmt --version | grep stable || echo "true"`
nightly_if_required := if is_rustfmt_nightly == "true" { "" } else { "+nightly" }

default:
  @just --list --justfile {{justfile()}}

prereqs:
  cd rust && cargo --version
  cd rust && cargo nextest --version
  cd rust && cargo audit --version
  cd rust && cargo clippy --version
  cd rust && cargo machete --help 2>&1 >/dev/null
  jq --version
  check-jsonschema --version
  hurl --version
  shellcheck --version

build:
  cd rust && cargo build --release

clean:
  cd rust && cargo clean
  rm -rf result database blocks staking-ledgers

format:
  cd rust && cargo {{nightly_if_required}} fmt --all

test: lint test-unit test-regression

test-unit:
  cd rust && cargo nextest run --release

test-unit-mina-rs:
  cd rust && cargo nextest run --release --features mina_rs

test-regression: build
  ./tests/regression

test-release: build
  ./tests/regression test_release

disallow-unused-cargo-deps:
  cd rust && cargo machete Cargo.toml

audit:
  cd rust && cargo audit

lint: && audit disallow-unused-cargo-deps
  shellcheck tests/regression
  shellcheck tests/stage-*
  shellcheck ops/productionize
  cd rust && cargo {{nightly_if_required}} fmt --all --check
  cd rust && cargo clippy --all-targets --all-features -- -D warnings
  [ "$(nixfmt < flake.nix)" == "$(cat flake.nix)" ]

# Build OCI images.
build-image:
  #REV="$(git rev-parse --short=8 HEAD)"
  #IMAGE=mina-indexer:"$REV"
  #docker --version
  #nix build .#dockerImage
  #docker load < ./result
  #docker run --rm -it "$IMAGE" \
  #  mina-indexer server start --help
  #docker image rm "$IMAGE"

# Start a server in the current directory.
start-server: build
  RUST_BACKTRACE=1 \
  ./rust/target/release/mina-indexer \
    --domain-socket-path ./mina-indexer.sock \
    server start \
      --log-level TRACE \
      --blocks-dir ./blocks \
      --staking-ledgers-dir ./staking-ledgers \
      --database-dir ./database

productionize: build
  ./ops/productionize

tier1-test: prereqs test

tier2-test: build
  tests/regression test_many_blocks
  # TODO: investigate failures in the following.
  # tests/regression test_many_blocks
  # TODO: re-enable once Nix build is working
  # nix build
  ops/ingest-all
