#!/usr/bin/env bash

set -eu
readonly RUSTY=./target/debug/rusty-cli
readonly RESTY=./resty-cli/bin/resty

readonly PATH_SAVE=$PATH

declare -a FAILED=()
declare -i RAN=0

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
            echo "$1"
        else
            echo "-----------------------"
        fi
    }

fi

expect() {
    local -r exp=$1
    shift

    local -r bin=$1
    shift

    local tmp; tmp=$(mktemp -d)

    "$bin" "$@" \
        >"$tmp/stdout" \
        2>"$tmp/stderr"

    local ec=$?

    local stdout; stdout=$(< "$tmp/stdout")
    local stderr; stderr=$(< "$tmp/stderr")

    rm -rf "$tmp"

    if [[ "$stdout" == "$exp" ]]; then
        echo "OK ($bin)"
        echo "  ARG       => $*"
        echo "  EXPECTED  => '$exp'"
        echo "  RESULT    => '$stdout'"
        echo "  STDERR    => '$stderr'"
        echo "  EXIT      => $ec"
    else
        echo "FAILURE ($bin)"
        echo "  ARG       => $*"
        echo "  EXPECTED  => '$exp'"
        echo "  RESULT    => '$stdout'"
        echo "  STDERR    => '$stderr'"
        echo "  EXIT      => $ec"
        return 1
    fi
}

expect-PATH() {
    local -r name=$1
    local -r expect=$2

    expect "$expect" type -p -f -P "$name" || return 1
}


expect-nginx() {
    local -r nginx=$1
    local -r resty=$2
    shift 2

    expect "$nginx" "$resty" -e 'return' "$@"
}

copy-binary() {
    local src=$1
    local dst=$2

    local parent; parent=$(dirname "$dst")
    mkdir -p "$parent"

    if [[ -e "$dst" ]]; then
        echo "$dst already exists"
        return 1
    fi

    cp --no-clobber --preserve=all "$src" "$dst"
}

setup-nginx() {
    local -r dst=$1
    copy-binary ./tests/print-argv0 "$dst"
}

symlink() {
    local -r target=$1
    local -r link_name=$2

    local parent=; parent=$(dirname "$link_name")
    mkdir -p "$parent"

    ln -sf "$target" "$link_name"
}

run-test() {
    local -r name=$1
    local -r func=$2

    local tmp; tmp=$(mktemp -d)
    log-group "$name"

    export PATH=$PATH_SAVE

    local test_bin=$RESTY
    echo "test bin => $test_bin"
    if ! "$func" "$tmp" "$test_bin"; then
        FAILED+=("$name ($test_bin)")
    fi

    export PATH=$PATH_SAVE

    rm -rf "$tmp"
    tmp=$(mktemp -d)

    test_bin=$RUSTY
    echo "test bin => $test_bin"
    if ! "$func" "$tmp" "$test_bin"; then
        FAILED+=("$name ($test_bin)")
    fi

    export PATH=$PATH_SAVE

    log-group

    rm -rf "$tmp"

    (( RAN++ )) || true

    return 0
}

test::explicit-nginx() {
    local -r tmp=$1
    local -r test_bin=$2

    local -r nginx="$tmp/bin/whatever/nginx"
    setup-nginx "$nginx"

    expect-nginx "$nginx" "$test_bin" --nginx "$nginx"
}

test::resty-nginx-sbin() {
    local -r tmp=$1
    local -r test_bin=$2

    local -r nginx="$tmp/openresty/bin/../nginx/sbin/nginx"

    setup-nginx "$nginx"
    local -r resty="$tmp/openresty/bin/resty"
    copy-binary "$test_bin" "$resty"
    expect-nginx "$nginx" "$resty"
}

test::resty-nginx-sbin-via-symlink() {
    local -r tmp=$1
    local -r test_bin=$2

    local -r nginx="$tmp/openresty/bin/../nginx/sbin/nginx"
    setup-nginx "$nginx"

    local resty="$tmp/openresty/bin/resty"
    copy-binary "$test_bin" "$resty"

    local resty_link="$tmp/symlinks-resty/resty"
    symlink "$resty" "$resty_link"

    expect-nginx "$nginx" "$resty_link"
}

test::resty-nginx-sbin-via-PATH-symlink() {
    local -r tmp=$1
    local -r test_bin=$2

    local -r nginx="$tmp/openresty/bin/../nginx/sbin/nginx"
    setup-nginx "$nginx"

    local resty=$tmp/openresty/bin/resty
    copy-binary "$test_bin" "$resty"

    local resty_link="$tmp/symlinks/resty"
    symlink "$resty" "$resty_link"

    expect-nginx "$nginx" "$resty_link"

    export PATH="$tmp/symlinks:$PATH"
    expect-PATH resty "$tmp/symlinks/resty"
    expect-nginx "$nginx" "resty"
}


test::resty-nginx-sbin-precedence-vs-sibling() {
    local -r tmp=$1
    local -r test_bin=$2

    local -r sibling="$tmp/openresty/bin/nginx"
    setup-nginx "$sibling"

    local resty=$tmp/openresty/bin/resty
    copy-binary "$test_bin" "$resty"

    # sibling
    expect-nginx "$sibling" "$resty"

    # ../nginx/sbin/nginx
    local -r nginx="$tmp/openresty/bin/../nginx/sbin/nginx"
    setup-nginx "$nginx"

    expect-nginx "$nginx" "$resty"
}

test::resty-nginx-sbin-precedence-vs-PATH() {
    local -r tmp=$1
    local -r test_bin=$2

    local -r path="$tmp/path-search"
    local -r in_path="$path/nginx"
    setup-nginx "$in_path"

    export PATH=${path}:$PATH
    expect-PATH nginx "$in_path"

    local resty=$tmp/openresty/bin/resty
    copy-binary "$test_bin" "$resty"

    # PATH
    expect-nginx "$in_path" "$resty"

    # ../nginx/sbin/nginx
    local -r nginx="$tmp/openresty/bin/../nginx/sbin/nginx"
    setup-nginx "$nginx"

    expect-PATH nginx "$in_path"

    expect-nginx "$nginx" "$resty"
}

test::sibling-nginx() {
    local -r tmp=$1
    local -r test_bin=$2

    local -r nginx="$tmp/openresty/bin/nginx"
    setup-nginx "$nginx"

    local resty=$tmp/openresty/bin/resty
    copy-binary "$test_bin" "$resty"

    expect-nginx "$nginx" "$resty"
}

test::sibling-nginx-precedence-vs-PATH() {
    local -r tmp=$1
    local -r test_bin=$2

    local -r sibling="$tmp/openresty/bin/nginx"

    # setup PATH version
    local -r path="$tmp/path-search"
    local -r in_path="$path/nginx"
    setup-nginx "$in_path"
    export PATH=${path}:$PATH
    expect-PATH nginx "$in_path"

    local resty=$tmp/openresty/bin/resty
    copy-binary "$test_bin" "$resty"

    # PATH first
    expect-nginx "$in_path" "$resty"

    # sibling takes precedence if it exists
    setup-nginx "$sibling"
    expect-PATH nginx "$in_path"
    expect-nginx "$sibling" "$resty"
}

test::sibling-nginx-via-symlink() {
    local -r tmp=$1
    local -r test_bin=$2

    local -r nginx="$tmp/openresty/bin/nginx"
    setup-nginx "$nginx"

    local resty=$tmp/openresty/bin/resty
    local resty_link="$tmp/symlinks-resty/resty"
    copy-binary "$test_bin" "$resty"
    symlink "$resty" "$resty_link"

    expect-nginx "$nginx" "$resty"
}

test::nginx-in-PATH() {
    local -r tmp=$1
    local -r test_bin=$2

    local -r path=$tmp/a/b/c
    local -r nginx="$path/nginx"
    setup-nginx "$nginx"

    local resty=$tmp/openresty/bin/resty
    copy-binary "$test_bin" "$resty"

    export PATH=${path}:$PATH
    expect-PATH nginx "$nginx"

    expect-nginx "$nginx" "$resty"
}

for test in $(compgen -A function test::); do
    name=${test#*::}
    run-test "$name" "$test"
done

if (( ${#FAILED[@]} > 0 )); then
    echo "Test failures:"
    for name in "${FAILED[@]}"; do
        echo "  - $name"
    done

    exit 1

elif (( RAN == 0 )); then
    echo "Something's wrong, no tests were executed?"
    exit 1

else
    echo "OK"
fi
