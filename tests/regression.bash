#!/usr/bin/env bash
# shellcheck disable=SC2046,SC2086,SC2002

# To debug, uncomment:
# set -x

set -euo pipefail

IDXR="$1"
shift

# Collect the binaries under test and the test ledgers.
SRC="$(git rev-parse --show-toplevel)"
REV="$(git rev-parse --short=8 HEAD)"
STAKING_LEDGERS="$SRC"/tests/data/staking_ledgers
SUMMARY_SCHEMA="$SRC"/tests/data/json-schemas/summary.json

# The rest of this script's logic assumes that the testing is done from within
# this temporary directory.
: "${VOLUMES_DIR:=/mnt}"
DEV_DIR="$VOLUMES_DIR"/mina-indexer-dev
BASE_DIR="$DEV_DIR"/rev-"$REV"
mkdir -p "$BASE_DIR"
cd "$BASE_DIR"

idxr() {
    RUST_BACKTRACE=full "$IDXR" --socket ./mina-indexer.sock "$@"
}

# Performs a shutdown of the Indexer, possibly returning failure.
#
shutdown_idxr() {
    echo "Shutting down Mina Indexer."

    if [ ! -S ./mina-indexer.sock ]; then
       echo "  Missing socket. Shutdown failed."
       return 1
    fi

    if [ ! -e ./idxr_pid ]; then
        echo "  Missing PID file. Shutdown failed."
        return 1
    fi

    if ! idxr server shutdown; then
      echo "  Server shutdown command failed."
      return 1
    fi

    # Shutdown command succeeded, but PID may still be active.
    if ! timeout 10s pwait -F ./idxr_pid; then
      # Either it timed out, or that PID did not exist.
      if ps -p "$(cat ./idxr_pid)" >/dev/null; then
        # The process still exists. The pwait must have timed out.
        echo "  The shutdown command did not kill the process. Failure."
        return 1
      else
        # The process does not exist. Check for socket deletion.
        if [ -S ./mina-indexer.sock ]; then
          echo "  The shutdown command did not delete the socket. Failure."
          return 1
        fi
      fi
    fi

    echo "Deleting PID file."
    rm -f ./idxr_pid
}

# Invoke this function when exiting this script for any reason.
#
cleanup() {
    err=$?
    if [ $err != 0 ]; then
        echo "Test failed ($test_name): $err"
        if ! shutdown_idxr; then
          # If there is a PID file, try to kill the process forcefully.
          if [ -e ./idxr_pid ]; then
            kill "$(cat ./idxr_pid)"
          fi
        fi
    fi
    exit "$err"
}

trap cleanup EXIT QUIT INT TERM

ephemeral_port() {
    set +e
    LOW_BOUND=49152
    RANGE=16384
    while true; do
        CANDIDATE=$(( LOW_BOUND + (RANDOM % RANGE) ))
        if ! (echo "" >/dev/tcp/127.0.0.1/${CANDIDATE}) >/dev/null 2>&1; then
            echo $CANDIDATE
            break
        fi
    done
    set -e
}

wait_for_socket() {
    num_retries=0
    # We retry 100 times before giving up because there is a good chance that
    # the machine is heavily loaded.
    while [ $num_retries -lt 100 ]; do
        if [ -S ./mina-indexer.sock ]; then
            return
        fi
        (( num_retries+=1 ))
        echo "Sleeping ($num_retries)..."
        sleep 1
    done
}

wait_forever_for_socket() {
    while true; do
        if [ -S ./mina-indexer.sock ]; then
            return
        fi
        echo "Sleeping 10 s ..."
        sleep 10
    done
}

idxr_server() {
    idxr server "$@" &
    echo $! > ./idxr_pid
}

idxr_server_start() {
    port=$(ephemeral_port)
    idxr_server start --web-port "$port" "$@"
}

idxr_server_start_standard() {
    idxr_server_start \
        --blocks-dir ./blocks \
        --staking-ledgers-dir ./staking-ledgers \
        --database-dir ./database \
        "$@"
}

idxr_server_sync() {
    port=$(ephemeral_port)
    idxr_server sync --web-port "$port" \
        --blocks-dir ./blocks \
        --staking-ledgers-dir ./staking-ledgers \
        "$@"
}

idxr_server_replay() {
    port=$(ephemeral_port)
    idxr_server replay --web-port "$port" \
        --blocks-dir ./blocks \
        --staking-ledgers-dir ./staking-ledgers \
        --database-dir ./database
}

stage_mainnet_blocks() {
    "$SRC"/ops/stage-blocks 2 "$1" mainnet "$2"
}

dl_mainnet_single() {
    "$SRC"/ops/stage-blocks "$1" "$1" mainnet "$2"
}

dl_mainnet_range() {
    "$SRC"/ops/stage-blocks "$1" "$2" mainnet "$3"
}

assert() {
    expected="$1"
    actual="$2"

    if [ "$expected" != "$actual" ]; then
        echo "  Test Failed: Expected $expected, but got $actual"
        exit 1
    else
        echo "  True: ${expected} = ${actual}"
    fi
}

assert_directory_exists() {
    directory="$1"

    if [ ! -d "$directory" ]; then
        echo "  Test Failed: Expected directory $directory to exist, but it does not."
        exit 1
    else
        echo "  True: Directory $directory exists."
    fi
}

teardown() {
    rm -f ./epoch_42_ledger.json
    rm -f ./epoch_0_staking_delegations.json
    rm -f ./epoch_0_ledger.json
    rm -rf ./blocks
    rm -rf ./staking-ledgers
    shutdown_idxr
    rm -rf ./database
    rm -f ./mina-indexer.sock
}

enter_test() {
  test_name="$1"
  echo
  echo "Test: $1"
}

test_indexer_cli_reports() {
    enter_test test_indexer_cli_reports

    # Indexer reports usage with no arguments
    ( "$IDXR" 2>&1 || true ) | grep -iq "Usage:"

    # Indexer reports usage for client subcommands
    idxr accounts --help 2>&1 |
        grep -iq "Usage: mina-indexer accounts"

    idxr accounts public-key --help 2>&1 |
        grep -iq "Usage: mina-indexer accounts public-key"

    idxr blocks --help 2>&1 |
        grep -iq "Usage: mina-indexer blocks"

    idxr blocks best-tip --help 2>&1 |
        grep -iq "Usage: mina-indexer blocks best-tip"

    idxr blocks height --help 2>&1 |
        grep -iq "Usage: mina-indexer blocks height"

    idxr blocks slot --help 2>&1 |
        grep -iq "Usage: mina-indexer blocks slot"

    idxr blocks public-key --help 2>&1 |
        grep -iq "Usage: mina-indexer blocks public-key"

    idxr blocks children --help 2>&1 |
        grep -iq "Usage: mina-indexer blocks children"

    idxr create-snapshot --help 2>&1 |
        grep -iq "Usage: mina-indexer create-snapshot"

    idxr restore-snapshot --help 2>&1 |
        grep -iq "Usage: mina-indexer restore-snapshot"

    idxr ledgers best --help 2>&1 |
        grep -iq "Usage: mina-indexer ledgers best"

    idxr ledgers --help 2>&1 |
        grep -iq "Usage: mina-indexer ledgers"

    idxr ledgers height --help 2>&1 |
        grep -iq "Usage: mina-indexer ledgers height"

    idxr staking-ledgers delegations --help 2>&1 |
        grep -iq "Usage: mina-indexer staking-ledgers delegations"

    idxr staking-ledgers public-key --help 2>&1 |
        grep -iq "Usage: mina-indexer staking-ledgers public-key"

    idxr staking-ledgers epoch --help 2>&1 |
        grep -iq "Usage: mina-indexer staking-ledgers epoch"

    idxr staking-ledgers hash --help 2>&1 |
        grep -iq "Usage: mina-indexer staking-ledgers hash"

    idxr server shutdown --help 2>&1 |
        grep -iq "Usage: mina-indexer server shutdown"

    idxr summary --help 2>&1 |
        grep -iq "Usage: mina-indexer summary"

    idxr transactions hash --help 2>&1 |
        grep -iq "Usage: mina-indexer transactions hash"

    idxr transactions public-key --help 2>&1 |
        grep -iq "Usage: mina-indexer transactions public-key"

    idxr transactions state-hash --help 2>&1 |
        grep -iq "Usage: mina-indexer transactions state-hash"

    idxr transactions state-hash --help 2>&1 |
        grep -iq "Usage: mina-indexer transactions state-hash"

    idxr internal-commands public-key --help 2>&1 |
        grep -iq "Usage: mina-indexer internal-commands public-key"

    idxr internal-commands state-hash --help 2>&1 |
        grep -iq "Usage: mina-indexer internal-commands state-hash"

    idxr snarks public-key --help 2>&1 |
        grep -iq "Usage: mina-indexer snarks public-key"

    idxr snarks state-hash --help 2>&1 |
        grep -iq "Usage: mina-indexer snarks state-hash"

    idxr version --help 2>&1 |
        grep -iq "Usage: mina-indexer version"
}

# Indexer server starts up without any precomputed blocks
test_server_startup() {
    enter_test test_server_startup

    idxr_server_start_standard
    wait_for_socket

    result=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    assert '3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ' $result

    teardown
}

# Indexer server ipc is available during initialization
test_ipc_is_available_immediately() {
    enter_test test_ipc_is_available_immediately

    stage_mainnet_blocks 100 ./blocks

    idxr_server_start_standard \
        --genesis-hash 3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ
    wait_for_socket

    idxr summary

    teardown
}

# Indexer server starts and creates directories with minimal args
test_startup_dirs_get_created() {
    enter_test test_startup_dirs_get_created

    idxr_server_start \
        --blocks-dir ./startup-blocks \
        --block-watch-dir ./watch-blocks \
        --staking-ledgers-dir ./startup-ledgers \
        --staking-ledger-watch-dir ./watch-ledgers \
        --database-dir ./database
    wait_for_socket

    assert_directory_exists "./startup-blocks"
    assert_directory_exists "./watch-blocks"
    assert_directory_exists "./startup-ledgers"
    assert_directory_exists "./watch-ledgers"
    assert_directory_exists "./database"

    teardown
}

# Indexer server reports correct balance for Genesis Ledger Account
test_account_balance_cli() {
    enter_test test_account_balance_cli

    idxr_server_start_standard
    wait_for_socket

    result=$(idxr accounts public-key --public-key B62qqDJCQsfDoHJvJCh1hgTpiVbmgBg8SbNKLMXsjuVsX5pxCELDyFk | jq -r .balance)
    assert '148837200000000' $result

    teardown
}

# Indexer server returns the correct account
test_account_public_key_json() {
    enter_test test_account_public_key_json

    idxr_server_start_standard
    wait_for_socket

    result=$(idxr accounts public-key --public-key B62qqDJCQsfDoHJvJCh1hgTpiVbmgBg8SbNKLMXsjuVsX5pxCELDyFk | jq -r .public_key)
    assert 'B62qqDJCQsfDoHJvJCh1hgTpiVbmgBg8SbNKLMXsjuVsX5pxCELDyFk' $result

    teardown
}

# Indexer summary returns the correct canonical root
test_canonical_root() {
    enter_test test_canonical_root

    stage_mainnet_blocks 15 ./blocks

    idxr_server_start_standard
    wait_for_socket

    hash=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    length=$(idxr summary --json | jq -r .witness_tree.canonical_root_length)

    assert 5 $length
    assert '3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY' $hash

    teardown
}

# Indexer server handles canonical threshold correctly
test_canonical_threshold() {
    enter_test test_canonical_threshold

    num_seq_blocks=15
    canonical_threshold=2

    stage_mainnet_blocks $num_seq_blocks ./blocks

    idxr_server_start_standard \
        --canonical-threshold $canonical_threshold
    wait_for_socket

    hash=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    length=$(idxr summary --json | jq -r .witness_tree.canonical_root_length)

    assert $(( num_seq_blocks - canonical_threshold )) $length
    assert '3NKXzc1hAE1bK9BSkJUhBBSznMhwW3ZxUTgdoLoqzW6SvqVFcAw5' $hash

    teardown
}

# Indexer server returns the correct best tip
test_best_tip() {
    enter_test test_best_tip

    stage_mainnet_blocks 15 ./blocks

    idxr_server_start_standard
    wait_for_socket

    # best tip query
    hash=$(idxr blocks best-tip | jq -r .state_hash)
    canonicity=$(idxr blocks best-tip | jq -r .canonicity)
    length=$(idxr blocks best-tip | jq -r .blockchain_length)
    canonicity_v=$(idxr blocks best-tip --verbose | jq -r .canonicity)

    # witness tree summary
    wt_hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    wt_length=$(idxr summary --json | jq -r .witness_tree.best_tip_length)

    assert $wt_hash $hash
    assert $wt_length $length
    assert 'Canonical' $canonicity
    assert 'Canonical' $canonicity_v

    teardown
}

# Indexer server returns the correct blocks for height and slot queries
test_blocks() {
    enter_test test_blocks

    stage_mainnet_blocks 10 ./blocks

    idxr_server_start_standard
    wait_for_socket

    # basic height query
    hash=$(idxr blocks height --height 10 | jq -r .[0].state_hash)
    slot=$(idxr blocks height --height 10 | jq -r .[0].global_slot_since_genesis)
    length=$(idxr blocks height --height 10 | jq -r .[0].blockchain_length)
    canonicity=$(idxr blocks height --height 10 | jq -r .[0].canonicity)

    # verbose height query
    hash_v=$(idxr blocks height --height 10 --verbose | jq -r .[0].state_hash)
    length_v=$(idxr blocks height --height 10 --verbose | jq -r .[0].blockchain_length)
    canonicity_v=$(idxr blocks height --height 10 --verbose | jq -r .[0].canonicity)

    # witness tree summary
    wt_hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    wt_length=$(idxr summary --json | jq -r .witness_tree.best_tip_length)

    assert 9 $slot
    assert $wt_hash $hash
    assert $wt_hash $hash_v
    assert $wt_length $length
    assert $wt_length $length_v
    assert 'Canonical' $canonicity
    assert 'Canonical' $canonicity_v

    # basic slot query
    hash=$(idxr blocks slot --slot 9 | jq -r .[0].state_hash)
    slot=$(idxr blocks slot --slot 9 | jq -r .[0].global_slot_since_genesis)
    length=$(idxr blocks slot --slot 9 | jq -r .[0].blockchain_length)
    canonicity=$(idxr blocks slot --slot 9 | jq -r .[0].canonicity)

    # verbose slot query
    hash_v=$(idxr blocks slot --slot 9 --verbose | jq -r .[0].state_hash)
    length_v=$(idxr blocks slot --slot 9 --verbose | jq -r .[0].blockchain_length)
    canonicity_v=$(idxr blocks slot --slot 9 --verbose | jq -r .[0].canonicity)

    assert 9 $slot
    assert $wt_hash $hash
    assert $wt_hash $hash_v
    assert $wt_length $length
    assert $wt_length $length_v
    assert 'Canonical' $canonicity
    assert 'Canonical' $canonicity_v

    # basic public key query
    hash=$(idxr blocks public-key --public-key B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3 | jq -r .[0].state_hash)
    slot=$(idxr blocks public-key --public-key B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3 | jq -r .[0].global_slot_since_genesis)
    length=$(idxr blocks public-key --public-key B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3 | jq -r .[0].blockchain_length)
    canonicity=$(idxr blocks public-key --public-key B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3 | jq -r .[0].canonicity)

    # verbose public key query
    hash_v=$(idxr blocks public-key --public-key B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3 --verbose | jq -r .[0].state_hash)
    length_v=$(idxr blocks public-key --public-key B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3 --verbose | jq -r .[0].blockchain_length)
    canonicity_v=$(idxr blocks public-key --public-key B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3 --verbose | jq -r .[0].canonicity)

    assert 9 $slot
    assert $wt_hash $hash
    assert $wt_hash $hash_v
    assert $wt_length $length
    assert $wt_length $length_v
    assert 'Canonical' $canonicity
    assert 'Canonical' $canonicity_v

    # height 10 = slot 9
    slot=$(idxr blocks slot --slot 9 | jq -r .)
    height=$(idxr blocks height --height 10 | jq -r .)
    assert "$slot" "$height"

    # write at height to file
    file=./blocks_at_height.json
    idxr blocks height --height 10 --path $file

    height=$(cat $file | jq -r .[0].blockchain_length)
    slot=$(cat $file | jq -r .[0].global_slot_since_genesis)

    assert 9 $slot
    assert 10 $height

    # write at slot to file
    file=./blocks_at_slot.json
    idxr blocks slot --slot 9 --path $file

    height=$(cat $file | jq -r .[0].blockchain_length)
    slot=$(cat $file | jq -r .[0].global_slot_since_genesis)

    assert 9 $slot
    assert 10 $height

    # write at public key to file
    file=./blocks_at_pk.json
    idxr blocks public-key --public-key B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3 --path $file

    height=$(cat $file | jq -r .[0].blockchain_length)
    slot=$(cat $file | jq -r .[0].global_slot_since_genesis)

    assert 9 $slot
    assert 10 $height

    teardown
}

# Indexer handles copied blocks correctly
test_block_copy() {
    enter_test test_block_copy

    stage_mainnet_blocks 10 ./blocks

    idxr_server_start_standard
    wait_for_socket

    # start without block 11
    best_hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    best_length=$(idxr summary --json | jq -r .witness_tree.best_tip_length)
    canonical_hash=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    canonical_length=$(idxr summary --json | jq -r .witness_tree.canonical_root_length)

    assert 10 $best_length
    assert 1 $canonical_length
    assert '3NKGgTk7en3347KH81yDra876GPAUSoSePrfVKPmwR1KHfMpvJC5' $best_hash
    assert '3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ' $canonical_hash

    # add block 11
    dl_mainnet_single 11 ./blocks
    sleep 1

    # check
    best_hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    best_length=$(idxr summary --json | jq -r .witness_tree.best_tip_length)
    canonical_hash=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    canonical_length=$(idxr summary --json | jq -r .witness_tree.canonical_root_length)

    assert 11 $best_length
    assert 1 $canonical_length
    assert '3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA' $best_hash
    assert '3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ' $canonical_hash

    teardown
}

# Indexer handles missing blocks correctly
test_missing_blocks() {
    enter_test test_missing_blocks

    stage_mainnet_blocks 10 ./blocks
    dl_mainnet_range 12 20 ./blocks # missing 11
    dl_mainnet_range 22 30 ./blocks # missing 21

    idxr_server_start_standard
    wait_for_socket

    # start out missing block 11 & 21
    num_dangling=$(idxr summary --json | jq -r .witness_tree.num_dangling)
    best_hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    best_length=$(idxr summary --json | jq -r .witness_tree.best_tip_length)
    canonical_hash=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    canonical_length=$(idxr summary --json | jq -r .witness_tree.canonical_root_length)

    assert 4 $num_dangling
    assert 10 $best_length
    assert 1 $canonical_length
    assert '3NKGgTk7en3347KH81yDra876GPAUSoSePrfVKPmwR1KHfMpvJC5' $best_hash
    assert '3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ' $canonical_hash

    # add missing block which connects the dangling branches
    dl_mainnet_single 21 ./blocks
    sleep 1

    # dangling branches combine
    # no new canonical blocks
    num_dangling=$(idxr summary --json | jq -r .witness_tree.num_dangling)
    best_hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    best_length=$(idxr summary --json | jq -r .witness_tree.best_tip_length)
    canonical_hash=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    canonical_length=$(idxr summary --json | jq -r .witness_tree.canonical_root_length)

    assert 3 $num_dangling
    assert 10 $best_length
    assert 1 $canonical_length
    assert '3NKGgTk7en3347KH81yDra876GPAUSoSePrfVKPmwR1KHfMpvJC5' $best_hash
    assert '3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ' $canonical_hash

    # add remaining missing block
    dl_mainnet_single 11 ./blocks
    sleep 1

    # dangling branches move into the root branch
    num_dangling=$(idxr summary --json | jq -r .witness_tree.num_dangling)
    best_hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    best_length=$(idxr summary --json | jq -r .witness_tree.best_tip_length)
    canonical_hash=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    canonical_length=$(idxr summary --json | jq -r .witness_tree.canonical_root_length)

    assert 0 $num_dangling
    assert 30 $best_length
    assert 20 $canonical_length
    assert '3NLpGhdRifLr31DGc61jUhsAdZiTy7EUw8cap41jrmzbTem5hc3V' $best_hash
    assert '3NLPpt5SyVnD1U5uJAqR3DL1Cqj5dG26SuWutRQ6AQpbQtQUWSYA' $canonical_hash

    teardown
}

# Indexer server returns the correct best chain
test_best_chain() {
    enter_test test_best_chain

    mkdir best_chain
    stage_mainnet_blocks 12 ./blocks

    idxr_server_start_standard
    wait_for_socket

    result=$(idxr chain best --num 1 | jq -r .[0].state_hash)
    assert '3NKkJDmNZGYdKVDDJkkamGdvNzASia2SXxKpu18imps7KqbNXENY' $result

    result=$(idxr chain best --verbose | jq -r .[0].state_hash)
    assert '3NKkJDmNZGYdKVDDJkkamGdvNzASia2SXxKpu18imps7KqbNXENY' $result

    # best chain with bounds
    bounds=$(idxr chain best \
        --start-state-hash 3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R \
        --end-state-hash 3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY \
        | jq -r .[0].state_hash)
    assert '3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY' $bounds

    bounds=$(idxr chain best \
        --start-state-hash '3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R' \
        --end-state-hash '3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY' \
        | jq -r .[1].state_hash)
    assert '3NL9qBsNibXPm5Nh8cSg5CCqrbzX5VUVY9gJzAbg7EVCF3hfhazG' $bounds

    bounds=$(idxr chain best --start-state-hash '3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R' \
        --end-state-hash '3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY' \
        | jq -r .[2].state_hash)
    assert '3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R' $bounds

    # write best chain to file
    file=./best_chain/best_chain.json
    idxr chain best --path $file
    file_result=$(cat $file | jq -r .[0].state_hash)
    assert '3NKkJDmNZGYdKVDDJkkamGdvNzASia2SXxKpu18imps7KqbNXENY' $file_result

    idxr chain best --verbose --path $file
    file_result=$(cat $file | jq -r .[0].state_hash)
    assert '3NKkJDmNZGYdKVDDJkkamGdvNzASia2SXxKpu18imps7KqbNXENY' $file_result

    rm -rf best_chain
    teardown
}

# Indexer server returns correct ledgers
test_ledgers() {
    enter_test test_ledgers

    mkdir ledgers
    stage_mainnet_blocks 15 ./blocks

    idxr_server_start_standard
    wait_for_socket

    pk='B62qp1RJRL7x249Z6sHCjKm1dbkpUWHRdiQbcDaz1nWUGa9rx48tYkR'

    # canonical ledgers match
    canonical_hash=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    canonical_height=$(idxr summary --json | jq -r .witness_tree.canonical_root_length)

    hash_balance=$(idxr ledgers hash --hash $canonical_hash | jq -r .${pk}.balance)
    height_balance=$(idxr ledgers height --height $canonical_height | jq -r .${pk}.balance)
    assert '607904750000000' $hash_balance
    assert '607904750000000' $height_balance

    # best ledgers match
    best_hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    best_height=$(idxr summary --json | jq -r .witness_tree.best_tip_length)

    best_result=$(idxr ledgers best | jq -r .${pk}.balance)
    hash_result=$(idxr ledgers hash --hash $best_hash | jq -r .${pk}.balance)
    height_balance=$(idxr ledgers height --height $best_height | jq -r .${pk}.balance)

    assert '607904750000000' $best_result
    assert '607904750000000' $hash_result
    assert '607904750000000' $height_balance

    # write ledgers to file
    file=./ledgers/best-block-$best_height-$best_hash.json
    idxr ledgers best --path $file

    file_result=$(cat $file | jq -r .${pk}.balance)
    assert '607904750000000' $file_result
    rm -f $file

    file=./ledgers/best-ledger-$best_height-$best_hash.json
    idxr ledgers hash --hash $best_hash --path $file

    file_result=$(cat $file | jq -r .${pk}.balance)
    assert '607904750000000' $file_result
    rm -f $file

    file=./ledgers/ledger-height-$best_height-$best_hash.json
    idxr ledgers height --height $best_height --path $file

    file_result=$(cat $file | jq -r .${pk}.balance)
    assert '607904750000000' $file_result
    rm -f $file

    rm -rf ledgers
    teardown
}

# Indexer server syncs with existing Speedb
test_sync() {
    enter_test test_sync

    stage_mainnet_blocks 15 ./blocks

    idxr_server_start_standard
    wait_for_socket

    idxr summary --verbose
    assert 26 $(idxr summary --json | jq -r .blocks_processed)
    shutdown_idxr

    # add more blocks to the watch dir while not indexing
    dl_mainnet_range 16 20 ./blocks

    # sync from previous indexer db
    idxr_server_sync --database-dir ./database
    wait_for_socket
    idxr summary --verbose

    # post-sync results
    sync_best_hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)

    # includes blocks added to watch dir while down
    assert 34 $(idxr summary --json | jq -r .blocks_processed)
    assert 20 $(idxr summary --json | jq -r .witness_tree.best_tip_length)
    assert '3NLPpt5SyVnD1U5uJAqR3DL1Cqj5dG26SuWutRQ6AQpbQtQUWSYA' $sync_best_hash

    teardown
}

# Indexer server replays events
test_replay() {
    enter_test test_replay

    stage_mainnet_blocks 15 ./blocks

    idxr_server_start_standard
    wait_for_socket

    assert 26 $(idxr summary --json | jq -r .blocks_processed)
    shutdown_idxr

    # add 8 more blocks to the watch dir while not indexing
    dl_mainnet_range 16 20 ./blocks

    # replay events from previous indexer instance & ingest the new blocks
    idxr_server_replay
    wait_for_socket

    # post-replay results
    root_hash_replay=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    best_hash_replay=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)

    assert 34 $(idxr summary --json | jq -r .blocks_processed)
    assert 20 $(idxr summary --json | jq -r .witness_tree.best_tip_length)
    assert 10 $(idxr summary --json | jq -r .witness_tree.canonical_root_length)
    assert '3NLPpt5SyVnD1U5uJAqR3DL1Cqj5dG26SuWutRQ6AQpbQtQUWSYA' $best_hash_replay
    assert '3NKGgTk7en3347KH81yDra876GPAUSoSePrfVKPmwR1KHfMpvJC5' $root_hash_replay

    teardown
}

# Indexer server returns correct transactions
test_transactions() {
    enter_test test_transactions

    mkdir transactions
    stage_mainnet_blocks 13 ./blocks

    idxr_server_start_standard
    wait_for_socket

    # basic pk transaction queries
    transactions=$(idxr transactions public-key --public-key B62qp1RJRL7x249Z6sHCjKm1dbkpUWHRdiQbcDaz1nWUGa9rx48tYkR | jq -r .)
    amount=$(idxr transactions public-key --public-key B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy | jq -r .[0].Payment.amount)

    assert '1000' $amount
    assert '[]' $transactions

    # basic pk transaction queries - verbose
    kind=$(idxr transactions public-key --public-key B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy --verbose | jq -r .[0].command.payload.body.kind)
    amount=$(idxr transactions public-key --public-key B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy --verbose | jq -r .[0].command.payload.body.amount)
    state_hash=$(idxr transactions public-key --public-key B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy --verbose | jq -r .[0].state_hash)
    tx_hash=$(idxr transactions public-key --public-key B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy --verbose | jq -r .[0].tx_hash)
    length=$(idxr transactions public-key --public-key B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy --verbose | jq -r .[0].blockchain_length)

    assert 3 $length
    assert '1000' $amount
    assert 'Payment' $kind
    assert '3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R' $state_hash
    assert 'CkpZirFuoLVVab6x2ry4j8Ld5gMmQdak7VHW6f5C7VJYE34WAEWqa' $tx_hash

    # bounded pk transaction queries
    amount=$(idxr transactions public-key --public-key B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy \
        --start-state-hash 3NL9qBsNibXPm5Nh8cSg5CCqrbzX5VUVY9gJzAbg7EVCF3hfhazG \
        --end-state-hash 3NKXzc1hAE1bK9BSkJUhBBSznMhwW3ZxUTgdoLoqzW6SvqVFcAw5 \
        | jq -r .[0].Payment.amount)
    assert '1000' $amount

    amount=$(idxr transactions public-key --public-key B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy \
        --start-state-hash 3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH \
        --end-state-hash 3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R \
        | jq -r .[0].Payment.amount)
    assert '1000' $amount

    # tx hash query
    amount=$(idxr transactions hash --hash CkpZirFuoLVVab6x2ry4j8Ld5gMmQdak7VHW6f5C7VJYE34WAEWqa | jq -r .Payment.amount)
    assert '1000' $amount

    # tx hash query - verbose
    kind=$(idxr transactions hash --hash CkpZirFuoLVVab6x2ry4j8Ld5gMmQdak7VHW6f5C7VJYE34WAEWqa --verbose | jq -r .command.payload.body.kind)
    amount=$(idxr transactions hash --hash CkpZirFuoLVVab6x2ry4j8Ld5gMmQdak7VHW6f5C7VJYE34WAEWqa --verbose | jq -r .command.payload.body.amount)
    status=$(idxr transactions hash --hash CkpZirFuoLVVab6x2ry4j8Ld5gMmQdak7VHW6f5C7VJYE34WAEWqa --verbose | jq -r .status.kind)
    tx_hash=$(idxr transactions hash --hash CkpZirFuoLVVab6x2ry4j8Ld5gMmQdak7VHW6f5C7VJYE34WAEWqa --verbose | jq -r .tx_hash)
    state_hash=$(idxr transactions hash --hash CkpZirFuoLVVab6x2ry4j8Ld5gMmQdak7VHW6f5C7VJYE34WAEWqa --verbose | jq -r .state_hash)
    length=$(idxr transactions hash --hash CkpZirFuoLVVab6x2ry4j8Ld5gMmQdak7VHW6f5C7VJYE34WAEWqa --verbose | jq -r .blockchain_length)

    assert 3 $length
    assert 'Payment' $kind
    assert '1000' $amount
    assert 'Failed' $status
    assert 'CkpZirFuoLVVab6x2ry4j8Ld5gMmQdak7VHW6f5C7VJYE34WAEWqa' $tx_hash
    assert '3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R' $state_hash

    # state hash query
    amount=$(idxr transactions state-hash --state-hash 3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R | jq -r .[0].Payment.amount)
    source=$(idxr transactions state-hash --state-hash 3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R | jq -r .[0].Payment.source)
    receiver=$(idxr transactions state-hash --state-hash 3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R | jq -r .[0].Payment.receiver)

    assert '1000' $amount
    assert 'B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy' $source
    assert 'B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM' $receiver

    # state hash query - verbose
    kind=$(idxr transactions state-hash --state-hash 3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R --verbose | jq -r .[0].data.kind)
    assert 'Signed_command' $kind

    amount=$(idxr transactions state-hash --state-hash 3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R --verbose | jq -r .[0].data.payload.body.amount)
    source=$(idxr transactions state-hash --state-hash 3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R --verbose | jq -r .[0].data.payload.body.source_pk)
    receiver=$(idxr transactions state-hash --state-hash 3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R --verbose | jq -r .[0].data.payload.body.receiver_pk)
    token=$(idxr transactions state-hash --state-hash 3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R --verbose | jq -r .[0].data.payload.body.token_id)

    assert '1' $token
    assert '1000' $amount
    assert 'B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy' $source
    assert 'B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM' $receiver

    # write transactions to file
    file=./transactions/transactions.json
    idxr transactions public-key --public-key B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy --path $file

    file_result=$(cat $file | jq -r .[0].Payment.amount)
    assert '1000' $file_result

    rm -rf ./transactions
    teardown
}

# Indexer server returns correct SNARK work
test_snark_work() {
    enter_test test_snark_work

    mkdir snark_work
    stage_mainnet_blocks 120 ./blocks

    idxr_server_start_standard \
        --canonical-threshold 5
    wait_for_socket

    # pk SNARK work queries
    # prover has SNARK work in block 111
    fee=$(idxr snarks public-key --public-key B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h | jq -r .[0].fee)
    prover=$(idxr snarks public-key --public-key B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h | jq -r .[0].prover)
    state_hash=$(idxr snarks public-key --public-key B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h | jq -r .[0].state_hash)

    assert '0' $fee
    assert 'B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h' $prover
    assert '3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK' $state_hash

    # state hash SNARK work queries
    fee=$(idxr snarks state-hash --state-hash 3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK | jq -r .[0].fee)
    prover=$(idxr snarks state-hash --state-hash 3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK | jq -r .[0].prover)

    assert '0' $fee
    assert 'B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h' $prover

    # write SNARK work from public key to file
    file=./snark_work/snark_work.json
    idxr snarks public-key --public-key B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h --path $file

    fee=$(cat $file | jq -r .[0].fee)
    prover=$(cat $file | jq -r .[0].prover)
    state_hash=$(cat $file | jq -r .[0].state_hash)

    assert '0' $fee
    assert 'B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h' $prover
    assert '3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK' $state_hash

    # write SNARK work from block to file
    file=./snark_work/snark_work.json
    idxr snarks state-hash --state-hash 3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK --path $file

    fee=$(cat $file | jq -r .[0].fee)
    prover=$(cat $file | jq -r .[0].prover)

    assert '0' $fee
    assert 'B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h' $prover

    # get top 5 SNARKers
    assert 0 $(idxr snarks top --num 5 | jq -r .[0].total_fees)
    assert 'B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h' $(idxr snarks top --num 5 | jq -r .[0].prover)

    rm -rf ./snark_work
    teardown
}

# Indexer server correctly creates a db snapshot
test_snapshot() {
    enter_test test_snapshot

    stage_mainnet_blocks 13 ./blocks

    idxr_server_start_standard
    wait_for_socket

    # pre-snapshot results
    canonical_hash=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    canonical_length=$(idxr summary --json | jq -r .witness_tree.canonical_root_length)
    best_hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    best_length=$(idxr summary --json | jq -r .witness_tree.best_tip_length)
    amount=$(idxr transactions public-key --public-key B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy --verbose | jq -r .[0].command.payload.body.amount)

    set -x
    # create snapshot
    idxr create-snapshot --output-path ./snapshot

    # kill running indexer and remove directories
    teardown

    idxr restore-snapshot --snapshot-file-path ./snapshot --restore-dir ./restore-path

    # sync a new indexer from snapshoted db
    idxr_server_sync --database-dir ./restore-path
    wait_for_socket

    # post-snapshot reults
    canonical_hash_snapshot=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    canonical_length_snapshot=$(idxr summary --json | jq -r .witness_tree.canonical_root_length)
    best_hash_snapshot=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    best_length_snapshot=$(idxr summary --json | jq -r .witness_tree.best_tip_length)
    amount_snapshot=$(idxr transactions public-key --public-key B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy --verbose | jq -r .[0].command.payload.body.amount)

    assert $canonical_hash $canonical_hash_snapshot
    assert $canonical_length $canonical_length_snapshot
    assert $best_hash $best_hash_snapshot
    assert $best_length $best_length_snapshot
    assert $amount $amount_snapshot

    rm -rf ./snapshot ./restore-path
    teardown
}

# Indexer server starts with many blocks
test_many_blocks() {
    enter_test test_many_blocks

    stage_mainnet_blocks 1000 ./blocks

    idxr_server_start_standard \
        --ledger-cadence 100
    wait_forever_for_socket

    # results
    best_hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    best_length=$(idxr summary --json | jq -r .witness_tree.best_tip_length)
    canonical_hash=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    canonical_length=$(idxr summary --json | jq -r .witness_tree.canonical_root_length)

    assert '1000' $best_length
    assert '990' $canonical_length
    assert '3NK9aySQJBEgAUKcWGrpbZhA4M8wL2N3cjipq3mEb4HPTuUkowEF' $canonical_hash
    assert '3NKrnCRmvomXqor8pnqrUsLv4XcofJBu8VWqAsWRirGNPszo1a66' $best_hash

    pk='B62qpJ4Q5J4LoBXgQBfq6gbXTyevFPhwMNYZEBdTSixmFq4UrdNadSN'

    # check ledgers are present
    # mainnet-100-3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4.json
    balance=$(idxr ledgers hash --hash 3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4 | jq -r .${pk}.balance)
    assert '502777775000000' $balance

    # mainnet-900-3NLHqp2mkmWbf4o69J4hg5cftRAAvZ5Edy7uqvJUUVvZWtD1xRrh.json
    balance=$(idxr ledgers hash --hash 3NLHqp2mkmWbf4o69J4hg5cftRAAvZ5Edy7uqvJUUVvZWtD1xRrh | jq -r .${pk}.balance)
    assert '502777775000000' $balance

    teardown
}

test_rest_accounts_summary() {
    enter_test test_rest_accounts_summary

    stage_mainnet_blocks 100 ./blocks
    port=$(ephemeral_port)

    idxr_server start \
        --blocks-dir ./blocks \
        --staking-ledgers-dir ./staking-ledgers \
        --database-dir ./database \
        --web-port "$port" \
        --web-hostname "0.0.0.0"
    wait_for_socket

    # results
    assert '1440050000000' $(curl --silent http://localhost:${port}/accounts/B62qrQBarKiVK11xP943pMQxnmNrfYpT7hskHLWdFXbx2K1E9wR1Vdy | jq -r .balance)
    assert 'B62qrQBarKiVK11xP943pMQxnmNrfYpT7hskHLWdFXbx2K1E9wR1Vdy' $(curl --silent http://localhost:${port}/accounts/B62qrQBarKiVK11xP943pMQxnmNrfYpT7hskHLWdFXbx2K1E9wR1Vdy | jq -r .delegate)
    assert '3' $(curl --silent http://localhost:${port}/accounts/B62qrQBarKiVK11xP943pMQxnmNrfYpT7hskHLWdFXbx2K1E9wR1Vdy | jq -r .nonce)

    # blocks
    assert '3' $(curl --silent http://localhost:${port}/accounts/B62qrQBarKiVK11xP943pMQxnmNrfYpT7hskHLWdFXbx2K1E9wR1Vdy | jq -r .epoch_num_blocks)
    assert '3' $(curl --silent http://localhost:${port}/accounts/B62qrQBarKiVK11xP943pMQxnmNrfYpT7hskHLWdFXbx2K1E9wR1Vdy | jq -r .total_num_blocks)

    # snarks
    assert '0' $(curl --silent http://localhost:${port}/accounts/B62qrQBarKiVK11xP943pMQxnmNrfYpT7hskHLWdFXbx2K1E9wR1Vdy | jq -r .epoch_num_snarks)
    assert '0' $(curl --silent http://localhost:${port}/accounts/B62qrQBarKiVK11xP943pMQxnmNrfYpT7hskHLWdFXbx2K1E9wR1Vdy | jq -r .total_num_snarks)

    # user commands
    assert '241' $(curl --silent http://localhost:${port}/accounts/B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy | jq -r .epoch_num_user_commands)
    assert '241' $(curl --silent http://localhost:${port}/accounts/B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy | jq -r .total_num_user_commands)

    # internal commands
    assert '2' $(curl --silent http://localhost:${port}/accounts/B62qmRG3THXszPjfJXDCk2MjDZqWLXMoVzyEWMPStEdfqhMe7GJaGxE | jq -r .epoch_num_internal_commands)
    assert '2' $(curl --silent http://localhost:${port}/accounts/B62qmRG3THXszPjfJXDCk2MjDZqWLXMoVzyEWMPStEdfqhMe7GJaGxE | jq -r .total_num_internal_commands)

    # Testing blockchain summary endpoint
    curl --silent http://localhost:${port}/summary > output.json

    blockchain_length=$(cat output.json | jq -r .blockchainLength)
    assert '100' $blockchain_length

    chain_id=$(cat output.json | jq -r .chainId)
    assert '5f704cc0c82e0ed70e873f0893d7e06f148524e3f0bdae2afb02e7819a0c24d1' $chain_id

    circulating_supply=$(cat output.json | jq -r .circulatingSupply)
    assert '89031537.840039233' $circulating_supply

    # date_time=$(cat output.json | jq -r .dateTime)
    # assert 'Wed, 17 Mar 2021 07:15:00 GMT' $date_time

    epoch=$(cat output.json | jq -r .epoch)
    assert '0' $epoch

    global_slot=$(cat output.json | jq -r .globalSlot)
    assert '145' $global_slot

    locked_supply=$(cat output.json | jq -r .lockedSupply)
    assert '716354155' $locked_supply

    min_window_density=$(cat output.json | jq -r .minWindowDensity)
    assert '77' $min_window_density

    next_epoch_ledger_hash=$(cat output.json | jq -r .nextEpochLedgerHash)
    assert 'jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee' $next_epoch_ledger_hash

    previous_state_hash=$(cat output.json | jq -r .previousStateHash)
    assert '3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh' $previous_state_hash

    slot=$(cat output.json | jq -r .slot)
    assert '145' $slot

    snarked_ledger_hash=$(cat output.json | jq -r .snarkedLedgerHash)
    assert 'jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee' $snarked_ledger_hash

    staged_ledger_hash=$(cat output.json | jq -r .stagedLedgerHash)
    assert 'jwWHNKQPdgLxHCsBki57a6zBfNPUFkAQmsrCNq3E7Q8oiNCNdkm' $staged_ledger_hash

    staking_epoch_ledger_hash=$(cat output.json | jq -r .stakingEpochLedgerHash)
    assert 'jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee' $staking_epoch_ledger_hash

    state_hash=$(cat output.json | jq -r .stateHash)
    assert '3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4' $state_hash

    total_currency=$(cat output.json | jq -r .totalCurrency)
    assert '805385692.840039233' $total_currency

    # block counts
    count=$(find ./blocks | wc -l) # leave the +1 for genesis block
    assert $count $(cat output.json | jq -r .epochNumBlocks)
    assert $count $(cat output.json | jq -r .totalNumBlocks)

    check-jsonschema --schemafile "$SUMMARY_SCHEMA" output.json

    teardown
}

test_rest_blocks() {
    enter_test test_rest_blocks

    stage_mainnet_blocks 100 ./blocks

    idxr_server_start_standard --web-hostname "0.0.0.0"
    wait_for_socket

    # /blocks endpoint
    curl --silent http://localhost:${port}/blocks > output.json
    assert $(idxr summary --json | jq -r .witness_tree.best_tip_hash) $(cat output.json | jq -r .[0].state_hash)

    # /blocks/{state_hash} endpoint
    curl --silent http://localhost:${port}/blocks/3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4 > output.json
    assert '3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4' $(cat output.json | jq -r .state_hash)

    # /blocks/height={height} endpoint
    curl --silent http://localhost:${port}/blocks/height=100 > output.json
    assert '3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4' $(cat output.json | jq -r .[0].state_hash)

    # /blocks/slot={globl_slot} endpoint
    curl --silent http://localhost:${port}/blocks/slot=145 > output.json
    assert '3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4' $(cat output.json | jq -r .[0].state_hash)

    teardown
}

test_release() {
    enter_test test_release

    stage_mainnet_blocks 9999 ./blocks

    idxr_server_start_standard
    wait_forever_for_socket

    # results
    best_hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    best_length=$(idxr summary --json | jq -r .witness_tree.best_tip_length)
    canonical_hash=$(idxr summary --json | jq -r .witness_tree.canonical_root_hash)
    canonical_length=$(idxr summary --json | jq -r .witness_tree.canonical_root_length)

    assert '9999' $best_length
    assert '9989' $canonical_length
    assert '3NKSj9ZRtZq9XCKkbf1a5s5YHWUMJkpLtp6irQKMPMKi24dU9yLi' $canonical_hash
    assert '3NLjanAoyqjqmPsQHafJcvQiGW2xbvyXANHxEyNwPwan2eUoZBV9' $best_hash

    pk='B62qpJ4Q5J4LoBXgQBfq6gbXTyevFPhwMNYZEBdTSixmFq4UrdNadSN'

    # check ledgers are present
    # mainnet-100-3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4.json
    balance=$(idxr ledgers hash --hash 3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4 | jq -r .${pk}.balance)
    assert '502777775000000' $balance

    # mainnet-900-3NLHqp2mkmWbf4o69J4hg5cftRAAvZ5Edy7uqvJUUVvZWtD1xRrh.json
    balance=$(idxr ledgers hash --hash 3NLHqp2mkmWbf4o69J4hg5cftRAAvZ5Edy7uqvJUUVvZWtD1xRrh | jq -r .${pk}.balance)
    assert '502777775000000' $balance

    teardown
}

test_genesis_block_creator() {
    enter_test test_genesis_block_creator

    idxr_server_start_standard
    wait_for_socket

    pk=B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg
    balance=$(idxr ledgers height --height 1 | jq -r .${pk}.balance)

    # verify that the genesis block winner account gets 1000 magic nanomina
    assert '1000' $balance

    teardown
}

test_txn_nonces() {
    enter_test test_txn_nonces

    stage_mainnet_blocks 100 ./blocks

    idxr_server_start_standard
    wait_for_socket

    pk=B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy

    # verify that the nonce matches what's reported from the archive node

    # after block 3
    nonce=$(idxr ledgers height --height 3 | jq -r .${pk}.nonce)
    assert '4' $nonce

    # after block 100
    nonce=$(idxr accounts public-key --public-key $pk | jq -r .nonce)
    assert '149' $nonce

    teardown
}

test_startup_staking_ledgers() {
    enter_test test_startup_staking_ledgers

    idxr_server_start \
        --blocks-dir ./blocks \
        --database-dir ./database \
        --staking-ledgers-dir $STAKING_LEDGERS
    wait_for_socket

    # epoch 0 staking ledger should be in the store, write it to a file
    idxr staking-ledgers epoch --epoch 0 --path ./epoch_0_ledger.json

    pk=B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg
    epoch0=jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee

    # check epoch 0 staking ledger values are correct
    voting_for=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.voting_for)
    receipt_chain_hash=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.receipt_chain_hash)

    assert 0 $(cat ./epoch_0_ledger.json | jq -r .epoch)
    assert 1 $(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.token)
    assert $pk $(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.pk)
    assert $pk $(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.delegate)
    assert 1000 $(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.balance)
    assert $epoch0 $(cat ./epoch_0_ledger.json | jq -r .ledger_hash)
    assert 'mainnet' $(cat ./epoch_0_ledger.json | jq -r .network)
    assert '3NK2tkzqqK5spR2sZ7tujjqPksL45M3UUrcA4WhCkeiPtnugyE2x' $voting_for
    assert '2mzbV7WevxLuchs2dAMY4vQBS6XttnCUF8Hvks4XNBQ5qiSGGBQe' $receipt_chain_hash

    # check summary staking epoch info
    max_staking_ledger_hash=$(idxr summary --json | jq -r .max_staking_ledger_hash)
    assert 42 $(idxr summary --json | jq -r .max_staking_ledger_epoch)
    assert 'jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH' $max_staking_ledger_hash

    teardown
}

test_watch_staking_ledgers() {
    enter_test test_watch_staking_ledgers

    idxr_server_start_standard \
        --staking-ledger-watch-dir ./staking-ledgers
    wait_for_socket

    # copy epoch 0 staking ledger from data to watched directory
    cp $STAKING_LEDGERS/mainnet-0-jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee.json ./staking-ledgers
    sleep 1

    # write epoch 0 ledger to file
    idxr staking-ledgers epoch --epoch 0 --path ./epoch_0_ledger.json

    # check account
    pk=B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg
    epoch0=jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee

    epoch=$(cat ./epoch_0_ledger.json | jq -r .epoch)
    network=$(cat ./epoch_0_ledger.json | jq -r .network)
    ledger_hash=$(cat ./epoch_0_ledger.json | jq -r .ledger_hash)
    token=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.token)
    public_key=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.pk)
    balance=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.balance)
    delegate=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.delegate)
    voting_for=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.voting_for)
    receipt_chain_hash=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.receipt_chain_hash)

    assert '1' $token
    assert '0' $epoch
    assert $pk $delegate
    assert $pk $public_key
    assert '1000' $balance
    assert 'mainnet' $network
    assert $epoch0 $ledger_hash
    assert '3NK2tkzqqK5spR2sZ7tujjqPksL45M3UUrcA4WhCkeiPtnugyE2x' $voting_for
    assert '2mzbV7WevxLuchs2dAMY4vQBS6XttnCUF8Hvks4XNBQ5qiSGGBQe' $receipt_chain_hash

    # Move epoch 42 staking ledger to watched directory
    epoch42=jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH
    cp "$STAKING_LEDGERS"/mainnet-42-"$epoch42".json ./staking-ledgers/
    sleep 1

    # write epoch 42 ledger to file
    idxr staking-ledgers epoch --epoch 42 --path ./epoch_42_ledger.json

    # check account
    pk=B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg

    epoch=$(cat ./epoch_42_ledger.json | jq -r .epoch)
    network=$(cat ./epoch_42_ledger.json | jq -r .network)
    ledger_hash=$(cat ./epoch_42_ledger.json | jq -r .ledger_hash)
    token=$(cat ./epoch_42_ledger.json | jq -r .staking_ledger.${pk}.token)
    public_key=$(cat ./epoch_42_ledger.json | jq -r .staking_ledger.${pk}.pk)
    balance=$(cat ./epoch_42_ledger.json | jq -r .staking_ledger.${pk}.balance)
    delegate=$(cat ./epoch_42_ledger.json | jq -r .staking_ledger.${pk}.delegate)
    voting_for=$(cat ./epoch_42_ledger.json | jq -r .staking_ledger.${pk}.voting_for)
    receipt_chain_hash=$(cat ./epoch_42_ledger.json | jq -r .staking_ledger.${pk}.receipt_chain_hash)

    assert '1' $token
    assert '42' $epoch
    assert $pk $delegate
    assert $pk $public_key
    assert '1000' $balance
    assert 'mainnet' $network
    assert $epoch42 $ledger_hash
    assert '3NK2tkzqqK5spR2sZ7tujjqPksL45M3UUrcA4WhCkeiPtnugyE2x' $voting_for
    assert '2mzbV7WevxLuchs2dAMY4vQBS6XttnCUF8Hvks4XNBQ5qiSGGBQe' $receipt_chain_hash

    teardown
}

test_staking_delegations() {
    enter_test test_staking_delegations

    idxr_server_start \
        --blocks-dir ./blocks \
        --database-dir ./database \
        --staking-ledgers-dir $STAKING_LEDGERS
    wait_for_socket

    # check account
    pk=B62qrxNgwAdhGYZv1BXQRt2HgopUceFyrtXZMikwsuaHu5FigRJjhwY
    epoch0=jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee

    epoch=$(idxr staking-ledgers public-key --epoch 0 --public-key $pk | jq -r .epoch)
    network=$(idxr staking-ledgers public-key --epoch 0 --public-key $pk | jq -r .network)
    public_key=$(idxr staking-ledgers public-key --epoch 0 --public-key $pk | jq -r .pk)
    total_stake=$(idxr staking-ledgers public-key --epoch 0 --public-key $pk | jq -r .total_stake)
    count_delegates=$(idxr staking-ledgers public-key --epoch 0 --public-key $pk | jq -r .count_delegates)
    total_delegated=$(idxr staking-ledgers public-key --epoch 0 --public-key $pk | jq -r .total_delegated)

    assert '0' $epoch
    assert $pk $public_key
    assert 'mainnet' $network
    assert '794268782956784283' $total_stake
    assert '2' $count_delegates
    assert '57617370302858700' $total_delegated

    # write aggregated delegations to file
    file=./epoch_0_staking_delegations.json
    idxr staking-ledgers delegations --epoch 0 --path ./epoch_0_staking_delegations.json

    # check account
    file_epoch=$(cat $file | jq -r .epoch)
    file_network=$(cat $file | jq -r .network)
    file_public_key=$(cat $file | jq -r .delegations.${pk}.pk)
    file_count_delegates=$(cat $file | jq -r .delegations.${pk}.count_delegates)
    file_total_delegated=$(cat $file | jq -r .delegations.${pk}.total_delegated)

    assert $epoch $file_epoch
    assert $pk $file_public_key
    assert $network $file_network
    assert $count_delegates $file_count_delegates
    assert $total_delegated $file_total_delegated

    teardown
}

test_internal_commands() {
    enter_test test_internal_commands

    stage_mainnet_blocks 11 ./blocks

    idxr_server_start_standard
    wait_for_socket

    pk=B62qs2YyNuo1LbNo5sbhPByDDAB7NZiejFM6H1ctND5ui7wH4PWa7qm
    block=3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA

    kind=$(idxr internal-commands public-key --public-key $pk | jq -r .[1].kind)
    amount=$(idxr internal-commands public-key --public-key $pk | jq -r .[1].amount)
    sender=$(idxr internal-commands public-key --public-key $pk | jq -r .[1].sender)
    receiver=$(idxr internal-commands public-key --public-key $pk | jq -r .[1].receiver)
    state_hash=$(idxr internal-commands public-key --public-key $pk | jq -r .[1].state_hash)

    assert $pk $receiver
    assert $block $state_hash
    assert '20000000' $amount
    assert 'Fee_transfer' $kind
    assert 'B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy' $sender

    kind=$(idxr internal-commands public-key --public-key $pk | jq -r .[0].kind)
    amount=$(idxr internal-commands public-key --public-key $pk | jq -r .[0].amount)
    receiver=$(idxr internal-commands public-key --public-key $pk | jq -r .[0].receiver)
    state_hash=$(idxr internal-commands public-key --public-key $pk | jq -r .[0].state_hash)

    assert $pk $receiver
    assert 'Coinbase' $kind
    assert $block $state_hash
    assert '720000000000' $amount

    # write internal commands for public key to file
    file=./internal_commands.json
    idxr internal-commands public-key --public-key $pk --path $file

    pk_kind=$(cat $file | jq -r .[0].kind)
    pk_amount=$(cat $file | jq -r .[0].amount)
    pk_receiver=$(cat $file | jq -r .[0].receiver)
    pk_state_hash=$(cat $file | jq -r .[0].state_hash)

    assert $amount $pk_amount
    assert 'Coinbase'  $pk_kind
    assert $receiver $pk_receiver
    assert $state_hash $pk_state_hash

    # write internal commands for block to file
    idxr internal-commands state-hash --state-hash $block --path $file

    hash_kind=$(cat $file | jq -r .[0].kind)
    hash_amount=$(cat $file | jq -r .[0].amount)
    hash_receiver=$(cat $file | jq -r .[0].receiver)
    hash_state_hash=$(cat $file | jq -r .[0].state_hash)

    assert $pk_kind $hash_kind
    assert $pk_amount $hash_amount
    assert $pk_receiver $hash_receiver
    assert $pk_state_hash $hash_state_hash

    teardown
}

# Indexer correctly starts from config
test_start_from_config() {
    enter_test test_start_from_config

    idxr_server_start_standard
    wait_for_socket
    teardown

    stage_mainnet_blocks 15 ./blocks

    port=$(ephemeral_port)
    file=./config.json
    echo "
    { \"genesis_hash\": \"3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ\",
      \"blocks_dir\": \"./blocks\",
      \"block_watch_dir\": \"./blocks\",
      \"staking_ledgers_dir\": \"./staking-ledgers\",
      \"staking_ledger_watch_dir\": \"./staking-ledgers\",
      \"database_dir\": \"./database\",
      \"log_level\": \"info\",
      \"ledger_cadence\": 100,
      \"reporting_freq\": 1000,
      \"prune_interval\": 10,
      \"canonical_threshold\": 10,
      \"canonical_update_threshold\": 2,
      \"web_hostname\": \"localhost\",
      \"web_port\": ${port},
      \"network\": \"mainnet\"
    }" > $file
    idxr_server start-via-config -p $file
    wait_for_socket

    hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    length=$(idxr summary --json | jq -r .witness_tree.best_tip_length)

    assert 15 $length
    assert '3NKkVW47d5Zxi7zvKufBrbiAvLzyKnFgsnN9vgCw65sffvHpv63M' $hash

    teardown
}

# Indexer correctly starts without blocks dir
test_start_without_blocks_dir() {
    enter_test test_start_without_blocks_dir

    idxr_server_start_standard --block-watch-dir ./blocks
    wait_for_socket

    stage_mainnet_blocks 15 ./blocks
    sleep 1

    hash=$(idxr summary --json | jq -r .witness_tree.best_tip_hash)
    length=$(idxr summary --json | jq -r .witness_tree.best_tip_length)

    assert 15 $length
    assert '3NKkVW47d5Zxi7zvKufBrbiAvLzyKnFgsnN9vgCw65sffvHpv63M' $hash

    teardown
}

# Indexer correctly starts without staking ledgers dir
test_start_without_ledgers_dir() {
    enter_test test_start_without_ledgers_dir

    idxr_server_start \
        --ledgers-watch-dir ./staking-ledgers \
        --database-dir ./database
    wait_for_socket

    # copy epoch 0 staking ledger from data to watched directory
    cp $STAKING_LEDGERS/mainnet-0-jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee.json ./staking-ledgers
    sleep 1

    # write epoch 0 ledger to file
    idxr staking-ledgers epoch --epoch 0 --path ./epoch_0_ledger.json

    # check account
    pk=B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg
    epoch0=jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee

    epoch=$(cat ./epoch_0_ledger.json | jq -r .epoch)
    network=$(cat ./epoch_0_ledger.json | jq -r .network)
    ledger_hash=$(cat ./epoch_0_ledger.json | jq -r .ledger_hash)
    token=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.token)
    public_key=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.pk)
    balance=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.balance)
    delegate=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.delegate)
    voting_for=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.voting_for)
    receipt_chain_hash=$(cat ./epoch_0_ledger.json | jq -r .staking_ledger.${pk}.receipt_chain_hash)

    assert '1' $token
    assert '0' $epoch
    assert $pk $delegate
    assert $pk $public_key
    assert '1000' $balance
    assert 'mainnet' $network
    assert $epoch0 $ledger_hash
    assert '3NK2tkzqqK5spR2sZ7tujjqPksL45M3UUrcA4WhCkeiPtnugyE2x' $voting_for
    assert '2mzbV7WevxLuchs2dAMY4vQBS6XttnCUF8Hvks4XNBQ5qiSGGBQe' $receipt_chain_hash

    teardown
}

test_clean_shutdown() {
    enter_test test_clean_shutdown

    idxr_server_start_standard
    wait_for_socket

    teardown  # Calls shutdown_idxr, which checks for clean shutdown.
}

test_clean_kill() {
    enter_test test_clean_kill

    idxr_server_start_standard
    wait_for_socket

    if [ ! -e ./idxr_pid ]; then
        echo "  Missing PID file. Cannot kill. Failure."
        return 1
    fi

    echo "  Sending Mina Indexer a SIGTERM"
    PID="$(cat ./idxr_pid)"
    kill "$PID"

    # We must give the process a chance to die, with 'pwait'.
    timeout 10s pwait -F ./idxr_pid || true

    # We waited with pwait. If the process is still there, it's a fail.
    if ps -p "$PID" >/dev/null; then
        echo "  The signal did not kill the process. Failure."
        return 1
    fi

    # Check for socket deletion.
    if [ -S ./mina-indexer.sock ]; then
        echo "  The signal handler did not delete the socket. Failure."
        return 1
    fi

    teardown
}

test_block_children() {
    enter_test test_block_children

    stage_mainnet_blocks 10 ./blocks

    idxr_server_start_standard --staking-ledger-watch-dir ./staking-ledgers
    wait_for_socket

    block_5_state_hash=3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY
    children=$(idxr blocks children --state-hash $block_5_state_hash)

    assert 6 $(echo "$children" | jq -r .[0].blockchain_length)
    assert 'Canonical' $(echo "$children" | jq -r .[0].canonicity)
    assert 5 $(echo "$children" | jq -r .[0].global_slot_since_genesis)
    assert $block_5_state_hash $(echo "$children" | jq -r .[0].parent_hash)
    assert '3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v' $(echo "$children" | jq -r .[0].state_hash)

    assert 6 $(echo "$children" | jq -r .[1].blockchain_length)
    assert 'Orphaned' $(echo "$children" | jq -r .[1].canonicity)
    assert 5 $(echo "$children" | jq -r .[1].global_slot_since_genesis)
    assert $block_5_state_hash $(echo "$children" | jq -r .[1].parent_hash)
    assert '3NKvdydTvLVDJ9PKAXrisjsXoZQvUy1V2sbComWyB2uyhARCJZ5M' $(echo "$children" | jq -r .[1].state_hash)

    assert 6 $(echo "$children" | jq -r .[2].blockchain_length)
    assert 'Orphaned' $(echo "$children" | jq -r .[2].canonicity)
    assert 5 $(echo "$children" | jq -r .[2].global_slot_since_genesis)
    assert $block_5_state_hash $(echo "$children" | jq -r .[2].parent_hash)
    assert '3NLM3k3Vk1qs36hZWdbWvi4sqwer3skbgPyHMWrZMBoscNLyjnY2' $(echo "$children" | jq -r .[2].state_hash)

    assert 6 $(echo "$children" | jq -r .[3].blockchain_length)
    assert 'Orphaned' $(echo "$children" | jq -r .[3].canonicity)
    assert 5 $(echo "$children" | jq -r .[3].global_slot_since_genesis)
    assert $block_5_state_hash $(echo "$children" | jq -r .[3].parent_hash)
    assert '3NKqMEewA8gvEiW7So7nZ3DN6tPnmCtHpWuAzADN5ff9wiqkGf45' $(echo "$children" | jq -r .[3].state_hash)

    teardown
}

test_hurl() {
    enter_test test_hurl

    stage_mainnet_blocks 120 ./blocks
    port=$(ephemeral_port)

    idxr_server start \
        --blocks-dir ./blocks \
        --staking-ledgers-dir $STAKING_LEDGERS \
        --database-dir ./database \
        --web-port "$port" \
        --web-hostname "0.0.0.0"
    wait_for_socket

    hurl --variable url=http://localhost:"$port"/graphql --test --parallel "$SRC"/tests/hurl/*.hurl

    teardown
}

test_version_file() {
    enter_test test_version_file

    idxr_server_start_standard
    wait_for_socket

    [ -e "./database/INDEXER_VERSION" ]
    teardown
}

test_missing_block_recovery() {
    enter_test test_missing_block_recovery

    stage_mainnet_blocks 5 ./blocks

    # start the indexer using the block recovery exe on path "$SRC"/tests/recovery.sh
    # wait for 3s in between recovery attempts
    idxr_server_start_standard \
        --missing-block-recovery-exe "$SRC"/tests/recovery.sh \
        --missing-block-recovery-delay 3 \
        --missing-block-recovery-batch true
    wait_for_socket

    # miss blocks at heights 6, 8, 11-16, 18-20
    dl_mainnet_single 7 ./blocks
    dl_mainnet_range 9 10 ./blocks
    dl_mainnet_single 17 ./blocks
    dl_mainnet_single 21 ./blocks

    # after blocks are added, check dangling branches
    sleep 1
    assert 8 $(idxr summary --json | jq -r .witness_tree.num_dangling)

    # wait for missing block recovery to work its magic
    sleep 30

    # check that all dangling branches have resolved & the best block has the right height
    assert 0 $(idxr summary --json | jq -r .witness_tree.num_dangling)
    assert 21 $(idxr summary --json | jq -r .witness_tree.best_tip_length)
    assert '3NKZ6DTHiMtuaeP3tJq2xe4uujVRnGT9FX1rBiZY521uNToSppUZ' $(idxr summary --json | jq -r .witness_tree.best_tip_hash)

    teardown
}

# ----
# Main
# ----
for test_name in "$@"; do
    case $test_name in
        "test_indexer_cli_reports") test_indexer_cli_reports ;;
        "test_server_startup") test_server_startup ;;
        "test_ipc_is_available_immediately") test_ipc_is_available_immediately ;;
        "test_startup_dirs_get_created") test_startup_dirs_get_created ;;
        "test_start_without_blocks_dir") test_start_without_blocks_dir ;;
        "test_start_without_ledgers_dir") test_start_without_ledgers_dir ;;
        "test_account_balance_cli") test_account_balance_cli ;;
        "test_account_public_key_json") test_account_public_key_json ;;
        "test_canonical_root") test_canonical_root ;;
        "test_canonical_threshold") test_canonical_threshold ;;
        "test_best_tip") test_best_tip ;;
        "test_blocks") test_blocks ;;
        "test_block_copy") test_block_copy ;;
        "test_missing_blocks") test_missing_blocks ;;
        "test_missing_block_recovery") test_missing_block_recovery ;;
        "test_best_chain") test_best_chain ;;
        "test_block_children") test_block_children ;;
        "test_ledgers") test_ledgers ;;
        "test_sync") test_sync ;;
        "test_replay") test_replay ;;
        "test_transactions") test_transactions ;;
        "test_snark_work") test_snark_work ;;
        "test_snapshot") test_snapshot ;;
        "test_rest_accounts_summary") test_rest_accounts_summary ;;
        "test_rest_blocks") test_rest_blocks ;;
        "test_genesis_block_creator") test_genesis_block_creator ;;
        "test_txn_nonces") test_txn_nonces ;;
        "test_startup_staking_ledgers") test_startup_staking_ledgers ;;
        "test_watch_staking_ledgers") test_watch_staking_ledgers ;;
        "test_staking_delegations") test_staking_delegations ;;
        "test_internal_commands") test_internal_commands ;;
        "test_start_from_config") test_start_from_config ;;
        "test_hurl") test_hurl ;;
        "test_clean_shutdown") test_clean_shutdown ;;
        "test_clean_kill") test_clean_kill ;;
        "test_version_file") test_version_file ;;
        # Tier 2 tests:
        "test_many_blocks") test_many_blocks ;;
        "test_release") test_release ;;
        *) echo "Unknown test: $test_name"
           exit 1
           ;;
    esac
done
