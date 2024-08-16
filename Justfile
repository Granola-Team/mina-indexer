# Justfile
#
# The command 'just' will give usage information.
# See https://github.com/casey/just for more.

export TOPLEVEL := `pwd`
export CARGO_HOME := TOPLEVEL + "/.cargo"
export GIT_COMMIT_HASH := `git rev-parse --short=8 HEAD`

IMAGE := "mina-indexer:" + GIT_COMMIT_HASH

alias c := check
alias tu := test-unit
alias tud := test-unit-dev
alias t1 := tier1
alias t2 := tier2
alias t3 := tier3

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
  ruby --version
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

lint:
  @echo "--- Linting ops scripts"
  ruby -cw ops/*.rb
  rubocop ops/*.rb
  shellcheck tests/regression.bash
  @echo "--- Linting Rust code"
  cd rust && time cargo {{nightly_if_required}} fmt --all --check
  cd rust && time cargo clippy --all-targets --all-features -- -D warnings
  @echo "--- Linting Nix configs"
  [ "$(nixfmt < flake.nix)" == "$(cat flake.nix)" ]
  @echo "--- Linting Cargo dependencies"
  cd rust && cargo machete Cargo.toml

nix-build:
  @echo "--- Performing Nix build"
  nom build

clean:
  cd rust && cargo clean
  rm -f result
  @echo "Consider also 'git clean -xdfn'"

format:
  cd rust && cargo {{nightly_if_required}} fmt --all

test-unit test='':
  @echo "--- Invoking 'cargo nextest'"
  cd rust && time cargo nextest run {{test}}

test-unit-dev: lint test-unit

# Perform a fast verification of whether the source compiles.
check:
  @echo "--- Invoking 'cargo check'"
  cd rust && time cargo check

test-unit-mina-rs:
  @echo "--- Performing long-running mina-rs unit tests"
  cd rust && time cargo nextest run --features mina_rs

# Perform a debug build
debug-build:
  cd rust && cargo build

# Quick debug-build and regression-test
bt subtest='': debug-build
  time ./ops/regression-test.rb "$TOPLEVEL"/rust/target/debug/mina-indexer {{subtest}}

# Quick (debug) unit-test and regression-test
tt subtest='': test-unit
  time ./ops/regression-test.rb "$TOPLEVEL"/rust/target/debug/mina-indexer {{subtest}}

# Build OCI images.
build-image:
  @echo "--- Building {{IMAGE}}"
  docker --version
  time nom build .#dockerImage
  time docker load < ./result
  docker run --rm -it {{IMAGE}} mina-indexer server start --help
  docker image rm {{IMAGE}}
  rm result

# Run the 1st tier of tests.
tier1: tier1-prereqs check lint test-unit
  @echo "--- Performing regressions tests subset"
  time ./ops/regression-test.rb "$TOPLEVEL"/rust/target/debug/mina-indexer \
    ipc_is_available_immediately \
    clean_shutdown \
    clean_kill \
    block_copy \
    account_balance_cli \
    best_chain \
    rest_accounts_summary \
    reuse_databases \
    hurl

load:
  @echo "--- Performing a simple load test'"
  time ./ops/regression-test.rb "$TOPLEVEL"/result/bin/mina-indexer load

# Run the 2nd tier of tests.
tier2: tier2-prereqs test-unit-mina-rs nix-build load && build-image
  @echo "--- Performing regressions test(s) with Nix-built binary"
  time ./ops/regression-test.rb "$TOPLEVEL"/result/bin/mina-indexer
  @echo "--- Performing many_blocks regression test"
  time ./ops/regression-test.rb "$TOPLEVEL"/result/bin/mina-indexer many_blocks
  @echo "--- Testing 'release'"
  time ./ops/regression-test.rb "$TOPLEVEL"/result/bin/mina-indexer release

# Run tier-3 tests.
tier3 blocks='5000': nix-build
  @echo "--- Performing tier3 tests"
  time ./ops/deploy.rb test {{blocks}}

# Run a server as if in production.
deploy-local-prod blocks='5000': nix-build
  @echo "--- Deploying to production"
  time ./ops/deploy.rb prod {{blocks}}

deploy-local-ci blocks='10000' web_port='8080': nix-build
  @echo "--- Deploying local CI instance"
  time ./ops/deploy.rb ci {{blocks}} {{web_port}}
