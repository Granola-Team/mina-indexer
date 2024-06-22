# Justfile
#
# The command 'just' will give usage information.
# See https://github.com/casey/just for more.

TOPLEVEL := `pwd`
export CARGO_HOME := TOPLEVEL + ".cargo"
export GIT_COMMIT_HASH := `git rev-parse --short=8 HEAD`

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
  @echo "--- Checking for tier-1 prereqs"
  cd rust && cargo --version
  cd rust && cargo nextest --version
  cd rust && cargo audit --version
  cd rust && cargo clippy --version
  cd rust && cargo machete --help 2>&1 >/dev/null
  shellcheck --version

tier2-prereqs: tier1-prereqs
  @echo "--- Checking for tier-2 prereqs"
  jq --version
  check-jsonschema --version
  hurl --version

audit:
  @echo "--- Performing Cargo audit"
  cd rust && time cargo audit

disallow-unused-cargo-deps:
  cd rust && cargo machete Cargo.toml

shellcheck:
  @echo "--- Linting shell scripts"
  ruby -cw ops/regression-test
  ruby -cw ops/deploy-local-prod
  ruby -cw ops/granola-rclone
  shellcheck tests/regression*
  shellcheck tests/stage-*
  shellcheck ops/deploy
  shellcheck ops/tier3-test

lint: shellcheck && audit disallow-unused-cargo-deps
  @echo "--- Linting"
  cd rust && time cargo {{nightly_if_required}} fmt --all --check
  cd rust && time cargo clippy --all-targets --all-features -- -D warnings
  [ "$(nixfmt < flake.nix)" == "$(cat flake.nix)" ]

nix-build:
  @echo "--- Performing Nix build"
  nom build

clean:
  cd rust && cargo clean
  rm -rf result database mina-indexer.sock

format:
  cd rust && cargo {{nightly_if_required}} fmt --all

test-unit:
  @echo "--- Performing unit tests"
  cd rust && time cargo nextest run

# Perform a fast verification of whether the source compiles.
check:
  @echo "--- Performing cargo check"
  cd rust && time cargo check

test-unit-mina-rs:
  @echo "--- Performing long-running mina-rs unit tests"
  cd rust && time cargo nextest run --features mina_rs

# Perform a debug build
debug-build:
  cd rust && cargo build

# Quick debug-build-and-regression-test
bt subtest='': debug-build
  time ./ops/regression-test {{TOPLEVEL}}/rust/target/debug/mina-indexer {{subtest}}

# Quick (debug) unit-test, and regression-test
tt subtest='': test-unit
  time ./ops/regression-test {{TOPLEVEL}}/rust/target/debug/mina-indexer {{subtest}}

test-regression subtest='':
  @echo "--- Performing regressions test(s)"
  time ./ops/regression-test {{TOPLEVEL}}/rust/target/debug/mina-indexer {{subtest}}

# Build OCI images.
build-image:
  @echo "--- Building {{IMAGE}}"
  docker --version
  time nom build .#dockerImage
  time docker load < ./result
  docker run --rm -it {{IMAGE}} mina-indexer server start --help
  docker image rm {{IMAGE}}
  rm result

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
deploy-local-prod: nix-build
  @echo "--- Deploying to production"
  time ./ops/deploy-local-prod

# Run the 1st tier of tests.
tier1: tier1-prereqs check lint test-unit test-regression

# Run the 2nd tier of tests.
tier2: tier2-prereqs test-unit-mina-rs nix-build && build-image
  @echo "--- Performing regressions test(s) with Nix-built binary"
  time ./ops/regression-test {{TOPLEVEL}}/result/bin/mina-indexer
  @echo "--- Performing test_many_blocks regression test"
  time ./ops/regression-test {{TOPLEVEL}}/result/bin/mina-indexer test_many_blocks
  @echo "--- Performing test_release"
  time ./ops/regression-test {{TOPLEVEL}}/result/bin/mina-indexer test_release

# Run tier-3 tests from './ops/deploy'.
tier3: nix-build
  @echo "--- Performing tier3 tests"
  time ./ops/tier3-test
