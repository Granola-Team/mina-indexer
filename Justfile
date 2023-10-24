# Justfile
#
# The command 'just' will give usage information.
# See https://github.com/casey/just for more.

default:
  @just --list --justfile {{justfile()}}

build:
  cargo build

clean:
  cargo clean

install-server:
  cargo install cargo-audit

test: test-unit test-regression

test-ci: lint test-unit test-regression

test-unit: build
  cargo nextest run

test-regression: build
  ./test

lint:
  cargo clippy -- -D warnings
  cargo clippy --all-targets --all-features -- -D warnings
  cargo check
  cargo audit

images:
  docker build .
