#!/usr/bin/env bash

set -euo pipefail
shopt -s failglob

readonly VERSIONS=./tests/versions
VERSION=${RESTY_CLI_COMPAT_VERSION:-latest}
CHECKOUT=0

set_version() {
    local version=${1:-$VERSION}

    if [[ $version == latest ]]; then
        local -a versions=("${VERSIONS}"/*)

        # strip leading path components
        versions=( "${versions[@]##*/}" )

        version=${versions[0]}

        local v
        for v in "${versions[@]}"; do
            if [[ $v > $VERSION ]]; then
                version=$v
            fi
        done
    else
        if [[ $version =~ ^[0-9]+$ ]]; then
            version="v0.${version}"
        fi

        CHECKOUT=1
    fi

    version=v${version#v}

    readonly VERSION=${version}
    readonly CHECKOUT
}

checkout() {
    if [[ ! -d resty-cli ]]; then
        git clone \
            https://github.com/openresty/resty-cli \
            resty-cli

    fi

    pushd resty-cli
    git reset --hard HEAD
    git clean -df
    git checkout .
    git fetch

    if (( CHECKOUT == 1 )); then
        echo "resty-cli: checkout $VERSION"
        git checkout "$VERSION"
    else
        echo "resty-cli: using current state ($(git rev-parse HEAD))"
    fi
    popd
}

setup_dirs() {
    rm -f ./t || true
    ln --no-target-directory -sf ./resty-cli/t ./t
}

patch_files() {
    local patch
    for patch in "${VERSIONS}/${VERSION}/patches"/*; do
        echo "patch: $patch"
        patch -p0 -i "$patch"
    done
}

main() {
    set_version "$@"
    checkout
    setup_dirs
    patch_files
}

main "$@"
