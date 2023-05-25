#!/usr/bin/env bash

set -u

readonly RUSTY=./target/debug/rusty-cli
readonly RESTY=./resty-cli/bin/resty

readonly TMP=$(mktemp -d)
readonly RESTY_JSON=$TMP/resty.json
readonly RUSTY_JSON=$TMP/rusty.json


if [[ ${CI:-} == "true" ]]; then
    readonly CI=1

    log-err() {
        echo "::error::$1"
    }

    log-group() {
        if [[ -n ${1:-} ]]; then
            echo "::group::$1"

        else
            echo "::endgroup::"
        fi
    }

else
    readonly CI=0

    log-err() {
        echo "$1"
    }

    log-group() {
        if [[ -n ${1:-} ]]; then
            echo "-----------------------"
        fi
    }

fi


run() {
    local -r name=$1
    shift

    local -a cmd=()

    if [[ $name == rusty ]]; then
        cmd+=( "$RUSTY" )
    else
        cmd+=( "$RESTY" )
    fi

    local first=${1:-}

    if (( $# > 0 && ${#first} > 0 )); then
        cmd+=( "$@" )
    fi

    cmd+=( "${ARGS[@]}" )

    env - PATH="$RUNNER_PATH" "${cmd[@]}" \
        > "$TMP/$name.stdout" \
        2> "$TMP/$name.stderr"

    echo "$?" > "$TMP/$name.exit_code"
}


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
    local -r exe=$1
    local -r fname=$2

    "$exe" ./tests/lua/nginx-conf-to-json.lua \
    | jq -S . \
    > "$fname"

    patch_result "$fname"

    jq -CS . < "$fname"
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

    log-group rusty
    generate "$RUSTY" "$RUSTY_JSON"
    log-group

    log-group resty
    generate "$RESTY" "$RESTY_JSON"
    log-group

    log-group result

    rc=0

    if diff "$RUSTY_JSON" "$RESTY_JSON"; then
        echo "OK!"
    else
        echo "FAILED!"
        rc=1
    fi

    log-group

    return "$rc"
}

main "$@"
