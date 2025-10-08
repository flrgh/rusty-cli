declare -g RUSTY=./target/debug/rusty-cli
declare -g RESTY=./resty-cli/bin/resty

declare -g RESTY_CLI_COMPAT_VERSION="${RESTY_CLI_COMPAT_VERSION:-0.29}"

declare -g TEST_ROOT
declare -g TMP

declare -g CI=${CI:-0}
if [[ $CI == "true" ]]; then
    CI=1
fi

if (( CI )); then
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
    GROUP=""

    log-err() {
        echo "$1"
    }

    log-group() {
        if [[ -n ${1:-} ]]; then
            GROUP=$1
            echo ">>> $GROUP >>>"
        else
            echo "<<< $GROUP <<<"
            GROUP=""
        fi
    }

fi


fatal() {
    log-err "FATAL: $1"
    exit 1
}

fail() {
    echo "FAILED: $1"
    return 1
}

_testing_cleanup() {
    if [[ -n ${TEST_ROOT:-} ]]; then
        rm -rf "${TEST_ROOT:?}"
        unset TEST_ROOT TMP TMPDIR
    fi
}

_testing_tmp_init() {
    unset TEST_ROOT TMP TMPDIR

    declare -g TEST_ROOT
    declare -g TMP
    declare -g TMPDIR

    TEST_ROOT=$(mktemp -d)
    TMP=${TEST_ROOT}/tmp
    mkdir "$TMP"

    TMPDIR=$TMP
    export TMPDIR

    trap _testing_cleanup ERR EXIT
}

declare -gi TESTS_RAN=0
testing_ran() {
    TESTS_RAN=$(( TESTS_RAN + 1 ))
}

testing_assert_tests_ran() {
    if (( TESTS_RAN < 1 )); then
        fatal "no tests were executed"
    fi
}

testing_init() {
    if [[ ! -x "$RESTY" ]]; then
        fatal "resty-cli executable not found at $RESTY"
    fi

    if [[ ! -x "$RUSTY" ]]; then
        fatal "rusty-cli executable not found at $RUSTY"
    fi

    _testing_tmp_init
}


testing_reset_temp() {
    if [[ -z ${TMP:-} ]]; then
        testing_init
    fi

    rm -rf "${TMP:?}"
    mkdir -p "${TMP:?}"
}

testing_exec() {
    local -r dir=${TMP}/exec
    rm -rf "$dir"
    mkdir -p "$dir"

    local -r cmd=$*
    echo "$cmd" > "${dir}.cmd"

    local -r stdout=${dir}.stdout
    local -r stderr=${dir}.stderr

    local -i ec=-1
    echo "$ec" > "${dir}.ec"

    local -i errexit=0
    if [[ $- = *e* ]]; then
        errexit=1
    fi

    set +e
    "$@" >"$stdout" 2>"$stderr"
    ec=$?
    set -e

    echo "$ec" > "${dir}.ec"

    declare -g -A REPLY=()
    REPLY[cmd]=$cmd
    REPLY[stdout]=$stdout
    REPLY[stderr]=$stderr
    REPLY[status]=$ec

    if (( ! errexit )); then
        set +e
    fi
}

assert_exec_ok() {
    local -r dir=${TMP}/exec

    if [[ ! -d $dir ]]; then
        fail "exec dir ($dir) not found"
        return 1
    fi

    local ec; ec=$(< "${dir}.ec")

    if ! [[ $ec =~ ^[0-9]+$ ]]; then
        fail "invalid exit code: $ec"
        return
    fi

    if (( ec != 0 )); then
        fail "non-zero exit code: $ec"
        return
    fi
}
