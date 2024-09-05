# Justfile
#
# The command 'just' will give usage information.
# See https://github.com/casey/just for more.

export TOPLEVEL := `pwd`
export CARGO_HOME := TOPLEVEL + "/.cargo"
export GIT_COMMIT_HASH := `git rev-parse --short=8 HEAD`

IMAGE := "mina-indexer:" + GIT_COMMIT_HASH

# Useful aliases
alias c := check
alias f := format
alias cd := clean-dev
alias tu := test-unit
alias t1 := tier1
alias t2 := tier2-dev
alias t3 := tier3-dev
alias dlp := deploy-local-prod-dev

# Ensure rustfmt works in all environments
# Nix environment has rustfmt nightly and won't work with +nightly
# Non-Nix environment needs nightly toolchain installed and requires +nightly
is_rustfmt_nightly := `cd rust && rustfmt --version | grep stable || echo "true"`
nightly_if_required := if is_rustfmt_nightly == "true" { "" } else { "+nightly" }

DEBUG_MODE := TOPLEVEL + "/rust/target/debug/mina-indexer"
PROD_MODE := TOPLEVEL + "/result/bin/mina-indexer"
REGRESSION_TEST := "./ops/regression-test.rb"
DEPLOY := "./ops/deploy.rb"

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
  #rubocop ops/*.rb
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

clean-dev:
  @echo "Cleaning dev directory"
  {{REGRESSION_TEST}} clean-dev

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
  time {{REGRESSION_TEST}} {{DEBUG_MODE}} {{subtest}}

# Quick debug-build and continue regression-test
ct subtest='': debug-build
  time {{REGRESSION_TEST}} {{DEBUG_MODE}} continue {{subtest}}

# Quick (debug) unit-test and regression-test
tt subtest='': test-unit
  time {{REGRESSION_TEST}} {{DEBUG_MODE}} {{subtest}}

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
  @echo "--- Performing tier 1 regression tests"
  time {{REGRESSION_TEST}} {{DEBUG_MODE}} \
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
  @echo "--- Performing a simple load test with Nix-built binary"
  time {{REGRESSION_TEST}} {{PROD_MODE}} load

load-dev:
  @echo "--- Performing a simple load test with debug-built binary"
  time {{REGRESSION_TEST}} {{DEBUG_MODE}} load

# Run the 2nd tier of tests with Nix-built binary.
tier2: tier2-prereqs nix-build load && build-image
  @echo "--- Performing tier 2 regression tests with Nix-built binary"
  time {{REGRESSION_TEST}} {{PROD_MODE}}
  @echo "--- Performing many_blocks regression test with Nix-built binary"
  time {{REGRESSION_TEST}} {{PROD_MODE}} many_blocks
  @echo "--- Performing release regression test with Nix-built binary"
  time {{REGRESSION_TEST}} {{PROD_MODE}} release

# Run the 2nd tier of with debug build.
tier2-dev: tier2-prereqs debug-build load-dev
  @echo "--- Performing tier 2 regression tests with debug-built binary"
  time {{REGRESSION_TEST}} {{DEBUG_MODE}}
  @echo "--- Performing many_blocks regression test with debug-built binary"
  time {{REGRESSION_TEST}} {{DEBUG_MODE}} many_blocks
  @echo "--- Performing release regression test with debug-built binary"
  time {{REGRESSION_TEST}} {{DEBUG_MODE}} release

# Run the 3rd tier of tests with Nix-built binary.
tier3 blocks='5000': test-unit-mina-rs nix-build
  @echo "--- Performing tier3 regression tests with Nix-built binary"
  time {{DEPLOY}} test nix {{blocks}}

# Run the 3rd tier of tests with debug build & no unit tests.
tier3-dev blocks='5000': debug-build
  @echo "--- Performing tier3 regression tests with debug-built binary"
  time {{DEPLOY}} test debug {{blocks}}

# Run a server as if in production with the Nix-built binary.
deploy-local-prod blocks='5000' web_port='': nix-build
  @echo "--- Deploying to production"
  time {{DEPLOY}} prod nix {{blocks}} {{web_port}}

# Run a server as if in production with the debug-built binary.
deploy-local-prod-dev blocks='5000' web_port='': debug-build
  @echo "--- Deploying to production dev"
  time {{DEPLOY}} prod debug {{blocks}} {{web_port}}
