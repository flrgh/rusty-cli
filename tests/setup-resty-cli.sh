#!/usr/bin/env bash

set -euo pipefail

if [[ ! -d resty-cli ]]; then
    git clone \
        https://github.com/openresty/resty-cli \
        resty-cli

fi

pushd resty-cli
git reset --hard HEAD
git checkout .
git fetch

if [[ -n ${RESTY_CLI_COMPAT_VERSION:-} ]]; then
    RESTY_CLI_COMPAT_VERSION=v${RESTY_CLI_COMPAT_VERSION#v}
    git checkout "$RESTY_CLI_COMPAT_VERSION"
fi
popd

rm ./t || true
ln --no-target-directory -sfv ./resty-cli/t ./t

for patch in ./tests/resty-cli-patches/*; do
    patch --verbose \
        -p0 \
        -i "$patch"
done
