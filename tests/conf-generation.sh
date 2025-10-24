#!/usr/bin/env bash

set -euo pipefail
source ./tests/lib.bash

testing_init

readonly RUSTY_JSON=${TMP}/rusty.json
readonly RESTY_JSON=${TMP}/resty.json


patch_result() {
    # the temp directory will obviously never match, so we need to
    # do some find/replace to patch out references to it
    sed -i -r \
        -e 's|/tmp/resty_[^/]+|RESTY_TEMP_DIR|g' \
        "$1"

    # the binary (arg[-1]) in each file will be different
    sed -i -r \
        -e "s|$RESTY|RESTY_BIN|g" \
        -e "s|$RUSTY|RESTY_BIN|g" \
        "$1"
}

generate() {
    local -r exe=${1:?}
    local -r fname=${2:?}
    shift 2

    testing_exec "$exe" "$@" ./tests/lua/nginx-conf-to-json.lua
    local stdout=${REPLY[stdout]}
    assert_exec_ok

    jq -S . \
        < "$stdout" \
        > "$fname"

    patch_result "$fname"

    jq -CS . < "$fname"
}


declare TEST
declare -a ARGS=()
declare -i PENDING=0

test::noargs() {
    TEST="no args"
    ARGS=()
}

test::load_module() {
    TEST="with --load-module <module>"
    ARGS=(
        --load-module "foobar"
    )
    PENDING=1
}

main() {
    if [[ ! -x "$RESTY" ]]; then
        log-err "fatal: resty-cli executable not found at $RESTY"
        exit 1
    fi

    if [[ ! -x "$RUSTY" ]]; then
        log-err "fatal: rusty-cli executable not found at $RUSTY"
        exit 1
    fi

    export RUSTY_STRIP_LUA_INDENT=1

    local fn
    local rc=0

    for fn in $(compgen -A function "test::"); do
        declare -g TEST="${fn#test::}"
        declare -g -a ARGS=()
        declare -g -i PENDING=0

        "$fn"

        if (( PENDING == 1 )); then
            log-group "${TEST} - pending"
            echo "test case is pending"
            log-group
            continue
        fi

        testing_reset_temp

        log-group "${TEST} - args"
        echo "${ARGS[*]}"
        log-group

        log-group "${TEST} - rusty"
        generate "$RUSTY" "$RUSTY_JSON" "${ARGS[@]}"
        log-group

        log-group "${TEST} - resty"
        generate "$RESTY" "$RESTY_JSON" "${ARGS[@]}"
        log-group

        log-group "${TEST} - result"
        if diff "$RUSTY_JSON" "$RESTY_JSON"; then
            echo "OK!"
        else
            echo "FAILED!"
            rc=1
        fi
        log-group

        testing_ran
    done

    testing_assert_tests_ran

    return "$rc"
}

main "$@"
