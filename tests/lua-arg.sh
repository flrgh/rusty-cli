#!/usr/bin/env bash

set -u
source ./tests/lib.bash
testing_init

readonly DIFF=$TMP/diff

readonly RUSTY_TMP=$TMP/rusty
readonly RESTY_TMP=$TMP/resty
mkdir -p "$RUSTY_TMP" "$RESTY_TMP"

readonly LUA_ARGV_FILE=./tests/lua/print-argv.lua
readonly LUA_ARGV_SCRIPT="dofile(\"$LUA_ARGV_FILE\")"

declare -g TEST
declare -g -a ARGS=()

test::basic() {
    TEST=basic
    ARGS=("LUA_ARGV")
}

test::prog_before() {
    TEST="-e <prog> before"
    ARGS=(-e 'tostring("BEFORE")' "LUA_ARGV")
}

test::prog_after() {
    TEST="-e <prog> after"
    ARGS=("LUA_ARGV" -e 'tostring("AFTER")')
}

test::prog_before_and_after() {
    TEST="-e <prog> before and and after"
    ARGS=(-e 'tostring("BEFORE")' "LUA_ARGV" -e 'tostring("AFTER")')
}

test::prog_before_combined() {
    TEST="-e=<prog> before"
    ARGS=(-e='tostring("BEFORE")' "LUA_ARGV")
}


test::more_args() {
    TEST="with more args"
    ARGS=(-I test -e 'tostring(1)' -c 10 "LUA_ARGV" --ns 1.2.3.4 -e 'tostring(2)')
}

# with user args
test::user_args() {
    TEST="user args"
    ARGS=("LUA_ARGV" a b c d)
}

test::user_args_terminated() {
    TEST="user args, terminated with --"
    ARGS=("LUA_ARGV" -- a b c d)
}


C=0

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

    env - \
        PATH="$RUNNER_PATH" \
        RESTY_CLI_COMPAT_VERSION="${RESTY_CLI_COMPAT_VERSION:-0.29}" \
        "${cmd[@]}" \
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
            fatal "FATAL: unexpected input: $name"
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
            log-err "FAIL: argv"
            echo
            cat "$DIFF"
            rc=1
        fi
    else
        rc=1
        log-err "FAIL: exit code mismatch"
        echo
        cat "$DIFF"
    fi


    if (( rc != 0 )); then
        debug_files rusty
        debug_files resty
        log-group
        exit 1
    fi

    return "$rc"
}

test_all() {
    local -r fn=${1:?}

    declare -g TEST="<empty>"
    declare -g -a ARGS=()
    "$fn"

    log-group "${TEST}"

    local file_args=()
    local inline_args=()
    local inline_eq_args=()

    for arg in "${ARGS[@]}"; do
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
    local fn
    for fn in $(compgen -A function "test::"); do
        test_all "$fn"
        testing_ran
    done

    testing_assert_tests_ran
}

main "$@"
