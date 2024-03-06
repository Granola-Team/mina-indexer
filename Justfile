# Justfile
#
# The command 'just' will give usage information.
# See https://github.com/casey/just for more.

default:
  @just --list --justfile {{justfile()}}

prereqs:
  cargo --version
  cargo nextest --version
  cargo audit --version
  cargo clippy --version
  cargo machete --help 2>&1 >/dev/null
  jq --version

build:
  cargo build --release

clean:
  cargo clean
  rm -rf result

format:
  cargo fmt --all

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
  cargo fmt --all --check
  cargo clippy --all-targets --all-features -- -D warnings

images:
  docker build .
