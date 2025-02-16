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
alias tu := test-unit
alias t1 := tier1
alias t2 := tier2
alias t3 := tier3-dev

# Prod
alias dlp := deploy-local-prod-dev

# Ensure rustfmt works in all environments
# Nix environment has rustfmt nightly and won't work with +nightly
# Non-Nix environment needs nightly toolchain installed and requires +nightly
is_rustfmt_nightly := `cd rust && rustfmt --version | grep stable || echo "true"`
nightly_if_required := if is_rustfmt_nightly == "true" { "" } else { "+nightly" }

BUILD_TYPE := "dev"
PROD_MODE := "nix"
REGRESSION_TEST := "./ops/regression-test.rb"
DEPLOY := "./ops/deploy.rb"
UTILS := "./ops/utils.rb"

default:
  @just --list --justfile {{justfile()}}

add crate='':
  cd rust && cargo add {{crate}}

rm crate='':
  cd rust && cargo rm {{crate}}

# Check for presence of tier 1 dependencies.
tier1-prereqs:
  @echo "--- Checking for tier-1 prereqs"
  ruby --version
  cd rust && cargo --version
  cd rust && cargo nextest --version
  cd rust && cargo audit --version
  cd rust && cargo clippy --version
  cd rust && cargo machete --help 2>&1 > /dev/null
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
  alejandra --check flake.nix ops/mina/mina_txn_hasher.nix
  @echo "--- Linting Rust code"
  cd rust && time cargo clippy --all-targets --all-features \
    -- \
    -Dwarnings \
    -Dclippy::too_many_lines \
    -Dclippy::negative_feature_names \
    -Dclippy::redundant_feature_names \
    -Dclippy::wildcard_dependencies \
    -Dclippy::unused_self \
    -Dclippy::used_underscore_binding \
    -Dclippy::zero_sized_map_values
  # Lints that demonstrably fail
  # -Dclippy::unused_async \
  # -Dclippy::multiple_crate_versions \
  # -Dclippy::cargo_common_metadata
  # -Dclippy::pedantic
  # -Dclippy::wildcard_imports
  @echo "--- Linting Cargo dependencies"
  cd rust && cargo machete

format:
  cd rust && cargo {{nightly_if_required}} fmt --all > /dev/null 2>&1
  # standardrb --fix ops/*.rb
  shfmt --write ops/*.sh 2>&1 >/dev/null
  alejandra flake.nix ops/mina/mina_txn_hasher.nix > /dev/null 2>&1

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

# Perform a dev build
dev-build:
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
show-dev which='one':
  @echo "Showing dev directory"
  {{UTILS}} dev show {{which}}

# Show prod directories
show-prod:
  @echo "Showing prod directory"
  {{UTILS}} prod show

# Show test directories
show-test:
  @echo "Showing test directory"
  {{UTILS}} test show

#
# Clean
#

# Cargo clean & remove nix build
clean:
  cd rust && cargo clean
  rm -f result
  @echo "Consider also 'git clean -xdfn'"

# Clean the mina-indexer-dev directory
clean-dev which='one':
  @echo "Cleaning dev directory"
  {{UTILS}} dev clean {{which}}

# Clean mina-indexer-prod subdirectory
clean-prod which='one':
  @echo "Cleaning prod directory"
  {{UTILS}} prod clean {{which}}

# Clean mina-indexer-test subdirectory
clean-test:
  @echo "Cleaning test directory"
  {{UTILS}} test clean

#
# Dev
#

# Download a mainnet PCB from the mina_network_block_data Google bucket
download-mina-block height state_hash dir='.':
  gsutil -m cp -n "gs://mina_network_block_data/mainnet-{{height}}-{{state_hash}}.json" {{dir}}

# Download mainnet PCBs from the mina_network_block_data Google bucket
download-mina-blocks height dir='.':
  gsutil -m cp -n "gs://mina_network_block_data/mainnet-{{height}}-*.json" {{dir}}

# Debug build and run regression tests
dev subtest='': dev-build
  time {{REGRESSION_TEST}} {{BUILD_TYPE}} {{subtest}}

# Debug build and continue regression tests from given test
dev-continue subtest='': dev-build
  time {{REGRESSION_TEST}} {{BUILD_TYPE}} continue {{subtest}}

#
# Unit tests
#

# Run unit tests 
test-unit-tier1 test='':
  @echo "--- Invoking 'rspec ops/spec'"
  rspec ops/spec/*_spec.rb
  @echo "--- Invoking 'cargo nextest'"
  cd rust && time cargo nextest run {{test}}

# Run release tier 2 & mina_rs unit test(s).
test-unit-tier2:
  @echo "--- Performing long-running unit tests"
  cd rust && time cargo nextest run --release --features "tier2 mina_rs"

# Run all unit tests of tier 2. (TODO: why this?)
test-unit:
  @echo "--- Performing all indexer unit tests"
  cd rust && time cargo nextest run --features tier2

#
# Tier 1 tests
#

# Run the 1st tier of tests.
tier1: tier1-prereqs lint test-unit-tier1
  @echo "--- Performing tier 1 regression tests"
  time {{REGRESSION_TEST}} {{BUILD_TYPE}} \
    ipc_is_available_immediately \
    clean_shutdown \
    clean_kill \
    block_copy \
    account_balance_cli \
    best_chain_v1 \
    rest_accounts_summary \
    reuse_databases \
    hurl_v1

#
# Tier 2 tests
#

# Run tier 2 load test
tier2-load-test:
  @echo "--- Performing a simple load test with dev-built binary"
  time {{REGRESSION_TEST}} {{BUILD_TYPE}} load_v1 \
  && {{REGRESSION_TEST}} {{BUILD_TYPE}} load_v2

# Run tier 2 best_chain_many_blocks test
tier2-best-chain-many-blocks-test:
  @echo "--- Performing best_chain_many_blocks regression test with dev-built binary"
  time {{REGRESSION_TEST}} {{BUILD_TYPE}} best_chain_many_blocks

# Run tier 2 regression tests.
tier2-regression-tests:
  @echo "--- Performing tier 2 regression tests with dev-built binary"
  time {{REGRESSION_TEST}} {{BUILD_TYPE}}

# Run tier 2 tests.
tier2: tier2-prereqs dev-build test-unit-tier2 \
  tier2-load-test \
  tier2-best-chain-many-blocks-test \
  tier2-regression-tests

#
# Tier 3 tests
#

# Run the 3rd tier of tests with Nix-built binary.
tier3 blocks='5000': nix-build && build-image
  @echo "--- Performing tier3 regression tests with Nix-built binary"
  time {{DEPLOY}} test nix {{blocks}}

# Run the 3rd tier of tests with dev build & no unit tests.
tier3-dev blocks='5000': dev-build
  @echo "--- Performing tier3 regression tests with dev-built binary"
  time {{DEPLOY}} test dev {{blocks}}

#
# Deploy local prod
#

# Run a server as if in production with the Nix-built binary.
deploy-local-prod blocks='5000' web_port='': nix-build
  @echo "--- Deploying prod indexer"
  time {{DEPLOY}} prod nix {{blocks}} {{web_port}}

# Run a server as if in production with the dev-built binary.
deploy-local-prod-dev blocks='5000' web_port='': dev-build
  @echo "--- Deploying dev prod indexer"
  time {{DEPLOY}} prod dev {{blocks}} {{web_port}}

# Shutdown a running local test/dev/prod indexer.
shutdown which='dev':
  @echo "Shutting down {{which}} indexer"
  {{UTILS}} {{which}} shutdown
  @echo "Successfully shutdown. You may also want to do 'just clean-{{which}}'"
