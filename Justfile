# Justfile
#
# The command 'just' will give usage information.
# See https://github.com/casey/just for more.

mod stage-blocks

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
DEPLOY_TIER3 := "./ops/deploy-tier3.rb"
DEPLOY_PROD := "./ops/deploy-prod.rb"
UTILS := "./ops/utils.rb"

default:
    @just --list --justfile {{ justfile() }}

add crate='':
    cd rust && cargo add {{ crate }}

rm crate='':
    cd rust && cargo rm {{ crate }}

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

lint: clippy
    @echo "--- Linting ops scripts"
    ruby -cw ops/*.rb
    standardrb --no-fix "ops/**/*.rb"
    @echo "--- Linting just recipes"
    just --fmt --check --unstable
    @echo "--- Linting regression scripts"
    shellcheck tests/regression.bash
    @echo "--- Linting Nix configs"
    alejandra --check flake.nix ops/mina/mina_txn_hasher.nix
    @echo "--- Linting Cargo dependencies"
    cd rust && cargo machete

clippy:
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

format:
    cd rust && cargo {{ nightly_if_required }} fmt --all > /dev/null 2>&1
    standardrb --fix "ops/**/*.rb"
    just --fmt --unstable
    shfmt --write ops/*.sh 2>&1 >/dev/null
    shfmt --write tests/*.sh 2>&1 >/dev/null
    shfmt --write tests/*.bash 2>&1 >/dev/null
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
    @echo "--- Building {{ IMAGE }}"
    docker --version
    time nom build .#dockerImage
    time docker load < ./result
    docker run --rm -it {{ IMAGE }} mina-indexer server start --help
    rm result

# Delete OCI image.
delete-image:
    @echo "--- Deleting OCI image {{ IMAGE }}"
    docker image rm {{ IMAGE }}

#
# Show
#

# Show mina-indexer PID(s)
show-pids:
    @echo "Showing mina-indexer PID(s)"
    {{ UTILS }} pids show

# Show the mina-indexer-dev directory
show-dev which='one':
    @echo "Showing dev directory"
    {{ UTILS }} dev show {{ which }}

# Show prod directories
show-prod:
    @echo "Showing prod directory"
    {{ UTILS }} prod show

# Show test directories
show-test:
    @echo "Showing test directory"
    {{ UTILS }} test show

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
    {{ UTILS }} dev clean {{ which }}

# Clean mina-indexer-prod subdirectory
clean-prod which='one':
    @echo "Cleaning prod directory"
    {{ UTILS }} prod clean {{ which }}

# Clean mina-indexer-test subdirectory
clean-test:
    @echo "Cleaning test directory"
    {{ UTILS }} test clean

#
# Dev
#

# Download a specific mainnet PCB (based on height and state hash) from o1Labs' bucket
download-mina-block height state_hash dir='.':
    ./ops/o1labs/download-mina-blocks.rb block {{ height }} {{ state_hash }} --dir {{ dir }}

# Download all mainnet PCBs (at a specific height) from o1Labs' bucket
download-mina-blocks height dir='.':
    ./ops/o1labs/download-mina-blocks.rb blocks {{ height }} --dir {{ dir }}

# Debug build and run regression tests
dev subtest='': dev-build
    time {{ REGRESSION_TEST }} {{ BUILD_TYPE }} {{ subtest }}

# Debug build and continue regression tests from given test
dev-continue subtest='': dev-build
    time {{ REGRESSION_TEST }} {{ BUILD_TYPE }} continue {{ subtest }}

#
# Unit tests
#

# Run unit tests.
test-unit-tier1 test='':
    @echo "--- Invoking 'rspec ops/spec'"
    rspec ops/spec/*_spec.rb
    @echo "--- Performing tier 1 unit test(s)"
    cd rust && time cargo nextest run {{ test }}

# Run all feature unit tests (debug build).
test-unit-tier2 test='':
    @echo "--- Performing all feature unit test(s)"
    cd rust && time cargo nextest run --all-features {{ test }}

#
# Tier 1 tests
#

# Run the 1st tier of tests.
tier1: tier1-prereqs lint test-unit-tier1
    @echo "--- Performing tier 1 regression tests"
    time {{ REGRESSION_TEST }} {{ BUILD_TYPE }} \
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

# Run regression test(s), either all of them or one named specific test.
regression-test subtest='':
    @echo "--- Performing regression tests {{ subtest }}"
    time {{ REGRESSION_TEST }} {{ BUILD_TYPE }} {{ subtest }}

# Run tier 2 tests.
tier2: tier2-prereqs test-unit-tier2 dev-build (regression-test "load_v1") (regression-test "load_v2") (regression-test "best_chain_many_blocks") regression-test

#
# Tier 3 tests
#

# Run the 3rd tier of tests with Nix-built binary.
tier3 blocks='5000': nix-build && build-image delete-image
    @echo "--- Performing tier3 regression tests with Nix-built binary"
    time {{ DEPLOY_TIER3 }} {{ PROD_MODE }} {{ blocks }}

# Run the 3rd tier of tests with dev build & no unit tests.
tier3-dev blocks='5000': dev-build
    @echo "--- Performing tier3 regression tests with dev-built binary"
    time {{ DEPLOY_TIER3 }} dev {{ blocks }}

#
# Deploy local prod
#

# Run a server as if in production with the Nix-built binary.
deploy-local-prod blocks='5000' web_port='': nix-build
    @echo "--- Deploying prod indexer"
    time {{ DEPLOY_PROD }} {{ PROD_MODE }} {{ blocks }} {{ web_port }}

# Run a server as if in production with the dev-built binary.
deploy-local-prod-dev blocks='5000' web_port='': (tier3-dev blocks)
    @echo "--- Deploying dev prod indexer"
    time {{ DEPLOY_PROD }} dev {{ blocks }} {{ web_port }}

# Shutdown a running local test/dev/prod indexer.
shutdown which='dev':
    @echo "Shutting down {{ which }} indexer"
    {{ UTILS }} {{ which }} shutdown
    @echo "Successfully shutdown. You may also want to do 'just clean-{{ which }}'"
