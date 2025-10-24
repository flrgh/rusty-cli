#!/usr/bin/env bash

set -euo pipefail
shopt -s failglob

readonly VERSIONS=./tests/versions

FORCE=0

detect_last_version() {
    local -r before=${1:?}

    local -a versions=("$VERSIONS"/*)

    # strip leading path components
    versions=( "${versions[@]##*/}" )

    local result
    local ver
    for ver in "${versions[@]}"; do
        if [[ $ver < $before && $ver > ${result:-"v0.0"} ]]; then
            result=$ver
        fi
    done

    if [[ -n ${result:-} ]]; then
        echo "last version before ${before}: ${result}"
        declare -g REPLY="$result"
    else
        echo "failed getting the most recent version before ${before}"
        return 1
    fi
}

patches() {
    local -r version=${1:?}
    local -r last=${2:?}

    mkdir -p "${VERSIONS}/${version}/patches"
    echo "creating patches in ${VERSIONS}/${version}/patches"
    ln \
        --relative \
        --symbolic \
        "${VERSIONS}/${last}/patches"/* \
        "${VERSIONS}/${version}/patches"
}

setup() {
    local -r version=${1:?}

    detect_last_version "$version"
    local -r last=$REPLY

    if (( FORCE )); then
        rm -rf "${VERSIONS:?}/${version:?}"
    fi

    patches "$version" "$last"
}

main() {
    local -a args=()
    while [[ -n ${1:-} ]]; do
        case $1 in
            -f|--force)
                FORCE=1
                ;;
            *)
                args+=("$1")
                ;;
        esac
        shift
    done

    set -- "${args[@]}"

    local version=${1:?version is required}
    version=v${version#v}

    if ! [[ $version =~ ^v[0-9]+\.[0-9]+$ ]]; then
        echo "error: invalid version: $version"
    fi

    setup "$version"
}

main "$@"
