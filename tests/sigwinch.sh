#!/usr/bin/env bash

set -euo pipefail

source ./tests/lib.bash
testing_init


test_it() {
    local -r bin=$1
    echo "TEST: $bin"

    local tmp; tmp=$(mktemp)

    "$bin" \
        -e "local fh = io.open('$tmp', 'w+'); fh:write(ngx.config.prefix()); fh:flush(); fh:close()" \
        -e 'ngx.sleep(60)' \
    &

    local pid=$!

    while [[ ! -s "$tmp" ]]; do
        kill -0 "$pid" || {
            fatal "$pid died while we were waiting"
        }
        sleep 1
    done

    local prefix; prefix=$(<"$tmp")
    if [[ -d $prefix ]]; then
        echo "prefix dir: $prefix"
    else
        fatal "nginx prefix dir ($prefix) does not exist at startup"
    fi

    echo -n "sending WINCH..."
    for _ in {1..20}; do
        echo -n .
        kill -WINCH "$pid"
        sleep 0.1
    done

    echo

    if [[ ! -d $prefix ]]; then
        fatal "nginx prefix dir ($prefix) does not exist after WINCH"
    fi

    echo "sending INT..."
    kill -INT "$pid"

    set +e
    wait "$pid"
    ec=$?
    set -e

    echo "$bin has exited with code: $ec"

    if [[ -d $prefix ]]; then
        fail "nginx prefix dir ($prefix) still exists after stopping"
        exit 1
    fi

    if (( ec != 130 )); then
        fail "unexpected exit code ($ec)"
        exit 1
    fi

    echo "OK"
}

test_it "$RESTY"
test_it "$RUSTY"
