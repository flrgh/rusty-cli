#!/usr/bin/env bash

set -u

readonly RUSTY=./target/debug/rusty-cli
readonly RESTY=./resty-cli/bin/resty

readonly TMP=$(mktemp -d)
readonly DIFF=$TMP/diff

trap "rm -rf $TMP" err exit

readonly RUSTY_TMP=$TMP/rusty
readonly RESTY_TMP=$TMP/resty
mkdir -p "$RUSTY_TMP" "$RESTY_TMP"

readonly LUA_ARGV_FILE=./tests/lua/print-argv.lua
readonly LUA_ARGV_SCRIPT="dofile(\"$LUA_ARGV_FILE\")"

# basic
readonly TEST_01=("LUA_ARGV")

# -e prog before/after
readonly TEST_02=(-e 'tostring("BEFORE")' "LUA_ARGV")
readonly TEST_03=(                        "LUA_ARGV" -e 'tostring("AFTER")')
readonly TEST_04=(-e 'tostring("BEFORE")' "LUA_ARGV" -e 'tostring("AFTER")')
readonly TEST_05=(-e='tostring("BEFORE")' "LUA_ARGV")

# more args
readonly TEST_06=(-I test -e 'tostring(1)' -c 10 "LUA_ARGV" --ns 1.2.3.4 -e 'tostring(2)')

# with user args
readonly TEST_07=("LUA_ARGV" a b c d)
readonly TEST_08=("LUA_ARGV" -- a b c d)

C=0

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

debug_files() {
    local name=$1

    echo
    echo "$name STDOUT:"
    echo
    cat  "$TMP/$name/stdout"
    echo
    echo "$name STDERR:"
    echo
    cat  "$TMP/$name/stderr"
    echo
}

generate() {
    local -r name=$1
    shift

    case "$name" in
        rusty)
            exe=$RUSTY
            ;;

        resty)
            exe=$RESTY
            ;;

        *)
            echo "FATAL: unexpected input: $name"
            exit 1
    esac

    local stdout=$TMP/${name}/stdout
    local stderr=$TMP/${name}/stderr
    local ec=$TMP/${name}/ec
    local argv=$TMP/${name}/argv

    : >"$stdout"
    : >"$stderr"
    : >"$ec"
    : >"$argv"

    RUSTY_CLI_TEST_OUTPUT="$argv" "$exe" "$@" \
        > "$stdout" \
        2> "$stderr"

    status=$?
    echo "$status" > "$ec"

    patch_result "$stdout"
    patch_result "$stderr"
    patch_result "$argv"

    cat "$argv"

    if (( status != 0 )); then
        echo "WARN: $name returned $status"
    fi
}

_diff() {
    diff -y "$1" "$2" > "$DIFF"
}

test_args() {
    echo "#################"
    echo "TEST #$(( ++C ))"
    echo "#################"

    echo
    echo "----- args ------"
    echo
    local i=0
    for arg in "$@"; do
        printf -- '%-10s = %s\n' "$(( ++i ))" "$arg"
    done
    echo

    echo "--- rusty-cli ---"
    echo
    generate rusty "$@"
    echo

    echo "--- resty-cli ---"
    echo
    generate resty "$@"
    echo

    local rc=0
    echo "---- result -----"
    echo

    if _diff "$TMP/rusty/ec" "$TMP/resty/ec"; then
        if _diff "$TMP/rusty/argv" "$TMP/resty/argv"; then
            echo "OK"
        else
            echo "FAIL: argv"
            echo
            cat "$DIFF"
            rc=1
        fi
    else
        rc=1
        echo "FAIL: exit code mismatch"
        echo
        cat "$DIFF"
    fi


    if (( rc != 0 )); then
        debug_files rusty
        debug_files resty
        exit 1
    fi

    return "$rc"
}

test_all() {
    local file_args=()
    local inline_args=()
    local inline_eq_args=()

    for arg in "$@"; do
        if [[ $arg == LUA_ARGV ]]; then
            file_args+=("$LUA_ARGV_FILE")
            inline_args+=(-e "$LUA_ARGV_SCRIPT")
            inline_eq_args+=("-e=$LUA_ARGV_SCRIPT")
        else
            file_args+=("$arg")
            inline_args+=("$arg")
            inline_eq_args+=("$arg")
        fi
    done

    test_args "${file_args[@]}"
    test_args "${inline_args[@]}"
    test_args "${inline_eq_args[@]}"
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

    test_all "${TEST_01[@]}"
    test_all "${TEST_02[@]}"
    test_all "${TEST_03[@]}"
    test_all "${TEST_04[@]}"
    test_all "${TEST_05[@]}"
    test_all "${TEST_06[@]}"
    test_all "${TEST_07[@]}"
    test_all "${TEST_08[@]}"
}

main "$@"
