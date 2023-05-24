#!/usr/bin/env bash

set -u

readonly -a ARGS=(
    -e 'print("hello")'
    --errlog-level=notice
    -I './path/to/directory'
    -e 'print(", world!")'
)

readonly RUNNER_OPTS='-a --flag --option "my quoted $value" --foo=bar a b c'

readonly RUSTY=./target/debug/rusty-cli
readonly RESTY=./resty-cli/bin/resty

readonly RUNNER_PATH=$PWD/tests/runners/bin:$PATH

readonly TMP=$(mktemp -d)

declare -a FAILED=()

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

    if (( $# > 0 && ${#1} > 0 )); then
        cmd+=( "$@" )
    fi
    cmd+=( "${ARGS[@]}" )

    env - PATH="$RUNNER_PATH" "${cmd[@]}" \
        > "$TMP/$name.stdout" \
        2> "$TMP/$name.stderr"

    echo "$?" > "$TMP/$name.exit_code"
}


patch_result_file() {
    # the temp directory will obviously never match, so we need to
    # do some find/replace to patch out references to it
    sed -i -r \
        -e 's|/tmp/resty_[^/]+|RESTY_TEMP_DIR|g' \
        "$1"
}


diff_result() {
    local -r case=$1
    local -r file=$2

    patch_result_file "$TMP/resty.${file}"
    patch_result_file "$TMP/rusty.${file}"

    printf '[%s] %s' \
        "$case" \
        "$file"

    if diff \
        "$TMP/resty.${file}" \
        "$TMP/rusty.${file}" \
        &> "$log"
    then
        printf ' OK\n'
        return 0
    else
        printf ' FAILED:\n'
        echo ">>>>>>>>>>>>"
        cat "$log"
        echo "<<<<<<<<<<<<"
        return 1
    fi
}

run_test() {
    local -r case=$1
    shift

    log-group "$case"

    run resty "$@"
    run rusty "$@"

    log=$TMP/diff.txt
    failed=0

    if ! diff_result "$case" exit_code; then
        failed=1
    fi

    if ! diff_result "$case" stdout; then
        failed=1
    fi

    if ! diff_result "$case" stderr; then
        failed=1
    fi

    if (( failed == 1 )); then
        FAILED+=( "$case" )
    fi

    log-group
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


    run_test default

    run_test gdb-no-opts   --gdb
    run_test gdb-with-opts --gdb --gdb-opts "$RUNNER_OPTS"

    run_test valgrind --valgrind
    run_test valgrind --valgrind --valgrind-opts "$RUNNER_OPTS"

    run_test stap-no-opts   --stap
    run_test stap-with-opts --stap --stap-opts "$RUNNER_OPTS"

    run_test user-no-opts   --user-runner custom-user-runner
    run_test user-with-opts --user-runner "custom-user-runner $RUNNER_OPTS"

    run_test rr --rr

    if (( ${#FAILED[@]} > 0 )); then
        log-group results

        log-err "There were ${#FAILED[@]} test failures:"

        printf -- '- %s\n' "${FAILED[@]}"

        log-group

        exit 1
    fi
}

main "$@"
