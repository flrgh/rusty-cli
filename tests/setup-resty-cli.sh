#!/usr/bin/env bash

set -euo pipefail

set -x

if [[ ! -d resty-cli ]]; then
    git clone \
        https://github.com/openresty/resty-cli \
        resty-cli
fi

ln --no-target-directory -sfv ./resty-cli/t ./t

for patch in ./tests/resty-cli-patches/*; do
    patch --verbose \
        -p0 \
        -i "$patch"
done
