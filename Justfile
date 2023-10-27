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

test: test-unit test-regression

test-ci: lint test-unit test-regression

test-unit: build
  # TODO: remove '--test-threads', which work around a bug in the test code.
  cargo nextest run --test-threads=1

test-regression: build
  ./test

audit:
  cargo audit

lint: && audit
  cargo clippy -- -D warnings
  cargo clippy --all-targets --all-features -- -D warnings
  cargo check

images:
  docker build .
