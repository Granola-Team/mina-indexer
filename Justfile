# Justfile
#
# The command 'just' will give usage information.
# See https://github.com/casey/just for more.

export CARGO_HOME := `pwd` + "/.cargo"
export GIT_COMMIT_HASH := `git rev-parse --short=8 HEAD`
IMAGE := "mina-indexer:" + GIT_COMMIT_HASH

# Prod

alias dlp := deploy-local-prod-dev

BUILD_TYPE := "dev"
PROD_MODE := "nix"
REGRESSION_TEST := "./ops/regression-test.rb"
DEPLOY_TIER3 := "./ops/deploy-tier3.rb"
DEPLOY_PROD := "./ops/deploy-prod.rb"
UTILS := "./ops/utils.rb"

default:
    @just --list --justfile {{ justfile() }}

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
