#!/usr/bin/env bash

set -euo pipefail
shopt -s extglob
shopt -s patsub_replacement

readonly FILES=(
    Cargo.toml
    .github/workflows/release.yml
)

readonly SEMVER='.*([0-9]+\.[0-9]+\.[0-9]+).*'

OLD_VERSION=
NEW_VERSION=
DIST=

# returns:
#   * 0 if there are any changes
#   * 1 if there are no changes
any-changed() {
    local _
    local -n __destvar=${1:-_}
    __destvar=$(git status --porcelain "${FILES[@]}")
    [[ -n ${__destvar:-} ]]
}

assert-clean-checkout() {
    local changes
    if any-changed changes; then
        echo "FATAL: cannot continue with uncomitted changes"
        echo "$changes"
        exit 1
    fi
}

detect-current-version() {
    OLD_VERSION=$(
        sed -n -r \
            -e "s/cargo-dist-version[ ]+=.*$SEMVER/\1/p" \
            < Cargo.toml
    )

    echo "current version in Cargo.toml: $OLD_VERSION"
}

detect-installed-dist-version() {
    cargo install cargo-dist &>/dev/null

    if command -v dist &>/dev/null; then
        echo "using 'dist' command"
        DIST=(dist)
        NEW_VERSION=$(dist --version)
    else
        echo "using 'cargo dist' command"
        DIST=(cargo dist)
        NEW_VERSION=$(cargo dist --version)
    fi

    # extract semver (e.g. `cargo-dist 1.2.3` => `1.2.3`)
    if [[ $NEW_VERSION =~ $SEMVER ]]; then
        NEW_VERSION=${BASH_REMATCH[1]}
        echo "installed version of cargo dist: $NEW_VERSION"
    else
        echo "FATAL: could not detect semver of cargo dist"
        exit 1
    fi
}

run-dist() {
    "${DIST[@]}" "$@"
}

update-cargo-toml() {
    echo "updating Cargo.toml"
    sed -i -r \
        -e "s/(cargo-dist-version)[ ]+=.*/\1 = \"$NEW_VERSION\"/g" \
        -e "s/(cargo-dist)[ ]+=.*/\1 = \"$NEW_VERSION\"/g" \
        Cargo.toml
}

update-release-workflow() {
    echo "updating release.yml workflow file"
    run-dist generate
}

do-git() {
    git checkout -b "chore/update-cargo-dist/$NEW_VERSION"
    git add "${FILES[@]}"
    git status
    git commit \
        -m "chore(deps): bump cargo-dist from $OLD_VERSION to $NEW_VERSION"
}

main() {
    assert-clean-checkout
    detect-current-version
    detect-installed-dist-version
    update-cargo-toml
    update-release-workflow

    if any-changed; then
        do-git
    else
        echo "no changes made"
    fi
}

main "$@"
