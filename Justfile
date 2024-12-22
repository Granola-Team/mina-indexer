# Justfile
#
# The command 'just' will give usage information.
# See https://github.com/casey/just for more.

export TOPLEVEL := `pwd`
export CARGO_HOME := TOPLEVEL + "/.cargo"
export GIT_COMMIT_HASH := `git rev-parse --short=8 HEAD`

IMAGE := "mina-indexer:" + GIT_COMMIT_HASH

# Show
alias sd := show-dev
alias sp := show-prod
alias st := show-test

# Clean
alias cd := clean-dev
alias cp := clean-prod
alias ct := clean-test

# Dev
alias bt := dev
alias btc := dev-continue

# Test
alias tu := test-unit-dev
alias t1 := tier1
alias t2 := tier2-dev
alias t3 := tier3-dev

# Prod
alias dlp := deploy-local-prod-dev

# Ensure rustfmt works in all environments
# Nix environment has rustfmt nightly and won't work with +nightly
# Non-Nix environment needs nightly toolchain installed and requires +nightly
is_rustfmt_nightly := `cd rust && rustfmt --version | grep stable || echo "true"`
nightly_if_required := if is_rustfmt_nightly == "true" { "" } else { "+nightly" }

DEBUG_MODE := "debug"
PROD_MODE := "nix"
REGRESSION_TEST := "./ops/regression-test.rb"
DEPLOY := "./ops/deploy.rb"
UTILS := "./ops/utils.rb"

default:
  @just --list --justfile {{justfile()}}

# Check for presence of tier 1 dependencies.
tier1-prereqs:
  @echo "--- Checking for tier-1 prereqs"
  ruby --version
  cd rust && cargo --version
  cd rust && cargo nextest --version
  cd rust && cargo audit --version
  cd rust && cargo clippy --version
  cd rust && cargo machete --help 2>&1 >/dev/null
  shellcheck --version
  shfmt --version

# Check for presence of tier 2 dependencies.
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
  standardrb --no-fix ops/*.rb
  shellcheck tests/regression.bash
  @echo "--- Linting Nix configs"
  alejandra --check flake.nix
  @echo "--- Linting Rust code"
  cd rust && time cargo {{nightly_if_required}} fmt --all --check
  cd rust && time cargo clippy --all-targets --all-features -- -D warnings
  @echo "--- Linting Cargo dependencies"
  cd rust && cargo machete Cargo.toml

format:
  cd rust && cargo {{nightly_if_required}} fmt --all
  standardrb --fix ops/*.rb
  shfmt --write ops/*.sh
  alejandra flake.nix

# Perform a fast verification of whether the source compiles.
check:
  @echo "--- Invoking 'cargo check'"
  cd rust && time cargo check

#
# Build
#

# Perform a nix (release) build
nix-build:
  @echo "--- Performing Nix build"
  nom build

# Perform a debug build
debug-build:
  cd rust && cargo build

# Build OCI images.
build-image:
  @echo "--- Building {{IMAGE}}"
  docker --version
  time nom build .#dockerImage
  time docker load < ./result
  docker run --rm -it {{IMAGE}} mina-indexer server start --help
  docker image rm {{IMAGE}}
  rm result

#
# Show
#

# Show mina-indexer PID(s)
show-pids:
  @echo "Showing mina-indexer PID(s)"
  {{UTILS}} pids show

# Show the mina-indexer-dev directory
show-dev rev=GIT_COMMIT_HASH:
  @echo "Showing dev directory"
  {{UTILS}} dev show {{rev}}

# Show prod directories
show-prod rev=GIT_COMMIT_HASH:
  @echo "Showing prod directory"
  {{UTILS}} prod show {{rev}}

# Show test directories
show-test rev=GIT_COMMIT_HASH:
  @echo "Showing test directory"
  {{UTILS}} test show {{rev}}

#
# Clean
#

# Cargo clean & remove nix build
clean:
  cd rust && cargo clean
  rm -f result
  @echo "Consider also 'git clean -xdfn'"

# Clean the mina-indexer-dev directory
clean-dev which='one' rev=GIT_COMMIT_HASH:
  @echo "Cleaning dev directory"
  {{UTILS}} dev clean {{which}} {{rev}}

# Clean mina-indexer-prod subdirectory
clean-prod which='one' rev=GIT_COMMIT_HASH:
  @echo "Cleaning prod directory"
  {{UTILS}} prod clean {{which}} {{rev}}

# Clean mina-indexer-test subdirectory
clean-test rev=GIT_COMMIT_HASH:
  @echo "Cleaning test directory"
  {{UTILS}} test clean {{rev}}

#
# Dev
#

# Download a mainnet PCB from the mina_network_block_data Google bucket
download-mina-block height state_hash dir='.':
  gsutil -m cp -n "gs://mina_network_block_data/mainnet-{{height}}-{{state_hash}}.json" {{dir}}

# Debug build and run regression tests
dev subtest='': debug-build
  time {{REGRESSION_TEST}} {{DEBUG_MODE}} {{subtest}}

# Debug build and continue regression tests from given test
dev-continue subtest='': debug-build
  time {{REGRESSION_TEST}} {{DEBUG_MODE}} continue {{subtest}}

#
# Unit tests
#

# Run unit debug tests
test-unit:
  @echo "--- Invoking 'rspec ops/spec'"
  rspec ops/spec/*_spec.rb
  @echo "--- Invoking 'cargo nextest'"
  cd rust && time cargo nextest run

# Lint & run debug unit test(s)
test-unit-dev test='': lint
  @echo "--- Invoking 'cargo nextest'"
  cd rust && time cargo nextest run {{test}}

test-unit-mina-rs:
  @echo "--- Performing long-running mina-rs unit tests"
  cd rust && time cargo nextest run --release --features mina_rs

#
# Tier 1 tests
#

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
    v2_signed_command_hash \
    hurl

# Tier 2 tests

# Run tier 2 nix (release) load test
tier2-load-test:
  @echo "--- Performing a simple load test with Nix-built binary"
  time {{REGRESSION_TEST}} {{PROD_MODE}} load

# Run tier 2 nix (release) best_chain_many_blocks test
tier2-best-chain-many-blocks-test:
  @echo "--- Performing best_chain_many_blocks regression test with Nix-built binary"
  time {{REGRESSION_TEST}} {{PROD_MODE}} best_chain_many_blocks

# Run tier 2 nix (release) regression tests
tier2-regression-tests:
  @echo "--- Performing tier 2 regression tests with Nix-built binary"
  time {{REGRESSION_TEST}} {{PROD_MODE}}

# Run tier 2 dev (debug) load test
tier2-load-test-dev:
  @echo "--- Performing a simple load test with debug-built binary"
  time {{REGRESSION_TEST}} {{DEBUG_MODE}} load

# Run tier 2 dev (debug) best_chain_many_blocks test
tier2-best-chain-many-blocks-test-dev:
  @echo "--- Performing best_chain_many_blocks regression test with debug-built binary"
  time {{REGRESSION_TEST}} {{DEBUG_MODE}} best_chain_many_blocks

# Run tier 2 dev (debug) regression tests
tier2-regression-tests-dev:
  @echo "--- Performing tier 2 regression tests with debug-built binary"
  time {{REGRESSION_TEST}} {{DEBUG_MODE}}

# Run tier 2 tests with Nix-built (release) binary & build OCI image.
tier2: tier2-prereqs nix-build \
  tier2-load-test \
  tier2-best-chain-many-blocks-test \
  tier2-regression-tests \
  && build-image

# Run tier 2 tests with debug build.
tier2-dev: tier2-prereqs debug-build \
  tier2-load-test-dev \
  tier2-best-chain-many-blocks-test-dev \
  tier2-regression-tests-dev

# Tier 3 tests

# Run the 3rd tier of tests with Nix-built binary.
tier3 blocks='5000': nix-build test-unit-mina-rs
  @echo "--- Performing tier3 regression tests with Nix-built binary"
  time {{DEPLOY}} test nix {{blocks}}

# Run the 3rd tier of tests with debug build & no unit tests.
tier3-dev blocks='5000': debug-build
  @echo "--- Performing tier3 regression tests with debug-built binary"
  time {{DEPLOY}} test debug {{blocks}}

#
# Deploy local prod
#

# Run a server as if in production with the Nix-built binary.
deploy-local-prod blocks='5000' web_port='': nix-build
  @echo "--- Deploying prod indexer"
  time {{DEPLOY}} prod nix {{blocks}} {{web_port}}

# Run a server as if in production with the debug-built binary.
deploy-local-prod-dev blocks='5000' web_port='': debug-build
  @echo "--- Deploying dev prod indexer"
  time {{DEPLOY}} prod debug {{blocks}} {{web_port}}

# Shutdown a running local prod indexer.
shutdown rev=GIT_COMMIT_HASH:
  @echo "Shutting down prod indexer"
  {{UTILS}} prod shutdown {{rev}}
  @echo "Successfully shutdown. You may also want to do 'just clean-prod'"
