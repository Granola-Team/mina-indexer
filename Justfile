# Justfile
#
# The command 'just' will give usage information.
# See https://github.com/casey/just for more.

export GIT_COMMIT_HASH := `git rev-parse --short=8 HEAD`
export CARGO_HOME := `pwd` + ".cargo"

IMAGE := "mina-indexer:" + GIT_COMMIT_HASH

# Ensure rustfmt works in all environments
# Nix environment has rustfmt nightly and won't work with +nightly
# Non-Nix environment needs nightly toolchain installed and requires +nightly
is_rustfmt_nightly := `cd rust && rustfmt --version | grep stable || echo "true"`
nightly_if_required := if is_rustfmt_nightly == "true" { "" } else { "+nightly" }

default:
  @just --list --justfile {{justfile()}}

# Check for presence of dev dependencies.
tier1-prereqs:
  echo "--- Checking for tier-1 prereqs"
  cd rust && cargo --version
  cd rust && cargo nextest --version
  cd rust && cargo audit --version
  cd rust && cargo clippy --version
  cd rust && cargo machete --help 2>&1 >/dev/null
  shellcheck --version

tier2-prereqs: tier1-prereqs
  echo "--- Checking for tier-2 prereqs"
  jq --version
  check-jsonschema --version
  hurl --version

audit:
  echo "--- Performing Cargo audit"
  cd rust && time cargo audit

disallow-unused-cargo-deps:
  cd rust && cargo machete Cargo.toml

shellcheck:
  echo "--- Linting shell scripts"
  shellcheck tests/regression
  shellcheck tests/stage-*
  shellcheck ops/productionize
  shellcheck ops/ingest-all

lint: shellcheck && audit disallow-unused-cargo-deps
  echo "--- Performing linting"
  cd rust && time cargo {{nightly_if_required}} fmt --all --check
  cd rust && time cargo clippy --all-targets --all-features -- -D warnings
  [ "$(nixfmt < flake.nix)" == "$(cat flake.nix)" ]

nix-build:
  echo "--- Performing Nix build"
  nix build

clean:
  cd rust && cargo clean
  rm -rf result database mina-indexer.sock

format:
  cd rust && cargo {{nightly_if_required}} fmt --all

test-unit:
  echo "--- Performing unit tests"
  cd rust && time cargo nextest run

# Perform a fast verification of whether the source compiles.
check:
  cd rust && time cargo check

test-unit-mina-rs:
  echo "--- Performing long-running mina-rs unit tests"
  cd rust && time cargo nextest run --features mina_rs

test-regression subtest='': nix-build
  echo "--- Performing regressions test(s)"
  time ./tests/regression {{subtest}}

# Build OCI images.
build-image:
  echo "--- Building {{IMAGE}}"
  docker --version
  time nix build .#dockerImage
  time docker load < ./result
  docker run --rm -it {{IMAGE}} \
    mina-indexer server start --help
  docker image rm {{IMAGE}}

# Start a server in the current directory.
start-server:
  cd rust && cargo build
  RUST_BACKTRACE=1 \
  ./rust/target/debug/mina-indexer \
    --socket ./mina-indexer.sock \
    server start \
      --log-level TRACE \
      --blocks-dir ./tests/data/initial-blocks \
      --staking-ledgers-dir ./tests/data/staking_ledgers \
      --database-dir ./database

# Delete the database created by 'start-server'.
delete-database:
  rm -fr ./database

# Run a server as if in production.
productionize: nix-build
  echo "--- Productionizing"
  time ./ops/productionize

# Run the 1st tier of tests.
tier1: tier1-prereqs check lint test-unit

# Run the 2nd tier of tests, ingesting blocks from /mnt/mina-logs...
tier2: test-regression tier2-prereqs nix-build test-unit-mina-rs build-image
  echo "--- Performing test_many_blocks regression test"
  time tests/regression test_many_blocks
  echo "--- Performing test_release"
  time tests/regression test_release
  echo "--- Ingesting all blocks..."
  time ops/ingest-all
