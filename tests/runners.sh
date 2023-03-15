#!/usr/bin/env bash


readonly -a ARGS=(
    -e 'print("hello")'
    --errlog-level=notice
    -I './path/to/directory'
    -e 'print(", world!")'
)

readonly RUNNER_OPTS='-a --flag --option "my quoted $value" --foo=bar a b c'

readonly RUSTY=./target/debug/rusty-cli
readonly RESTY=./bin/resty.pl

readonly RUNNER_PATH=$PWD/tests/runners/bin:$PATH

readonly TMP=$(mktemp -d)


readonly TEST_DEFAULT=()

readonly TEST_GDB=( --gdb )
readonly TEST_GDB_WITH_OPTS=( --gdb --gdb-opts "$RUNNER_OPTS" )

readonly TEST_VALGRIND=( --valgrind )
readonly TEST_VALGRIND_WITH_OPTS=( --valgrind --valgrind-opts "$RUNNER_OPTS" )

readonly TEST_STAP=( --stap )
readonly TEST_STAP_WITH_OPTS=( --stap --stap-opts "$RUNNER_OPTS" )

readonly TEST_USER=( --user-runner custom-user-runner )
readonly TEST_USER_WITH_OPTS=( --user-runner "custom-user-runner $RUNNER_OPTS" )

readonly TEST_RR=( --rr )

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

declare -a FAILED=()

diff_result() {
    local -r case=$1
    local -r file=$2

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

for varname in ${!TEST_*}; do
    case=${varname#TEST_}

    declare -n var="$varname"

    echo "-----------------------"

    run resty "${var[@]}"
    run rusty "${var[@]}"

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
done

if (( ${#FAILED[@]} > 0 )); then
    echo "There were ${#FAILED[@]} test failures:"
    printf -- '- %s\n' "${FAILED[@]}"
    exit 1
fi
