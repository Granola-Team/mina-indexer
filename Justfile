# Justfile
#
# The command 'just' will give usage information.
# See https://github.com/casey/just for more.

# Ensure rustfmt works in all environments
# Nix environment has rustfmt nightly and won't work with +nightly
# Non-Nix environment needs nightly toolchain installed and requires +nightly
is_rustfmt_nightly := `rustfmt --version | grep stable || echo "true"`
nightly_if_required := if is_rustfmt_nightly == "true" { "" } else { "+nightly" }

default:
  @just --list --justfile {{justfile()}}

prereqs:
  cargo --version
  cargo nextest --version
  cargo audit --version
  cargo clippy --version
  cargo machete --help 2>&1 >/dev/null
  jq --version
  check-jsonschema --version

build:
  cargo build --release

clean:
  cargo clean
  rm -rf result

format:
  cargo {{nightly_if_required}} fmt --all

test: test-unit
  ./test

test-ci: lint test-unit
  ./test

test-unit: build
  cargo nextest run --release

test-regression: build
  ./test

test-release: build
  ./test test_release

disallow-unused-cargo-deps:
  cargo machete Cargo.toml

audit:
  cargo audit

lint: && audit disallow-unused-cargo-deps
  cargo {{nightly_if_required}} fmt --all --check
  cargo clippy --all-targets --all-features -- -D warnings

images:
  docker build .
