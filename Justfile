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
  command -v pgrep

build:
  cargo build

clean:
  cargo clean
  rm -rf result

test: test-unit test-regression

test-ci: lint test-unit test-regression

test-unit: build
  # TODO: remove '--test-threads', which work around a bug in the test code.
  cargo nextest run --test-threads=1

test-regression: build
  ./test

disallow-unused-cargo-deps:
  cargo machete Cargo.toml

audit:
  cargo audit

lint: && audit disallow-unused-cargo-deps
  cargo clippy -- -D warnings
  cargo clippy --all-targets --all-features -- -D warnings
  cargo check

images:
  docker build .
