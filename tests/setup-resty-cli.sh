#!/usr/bin/env bash

set -euo pipefail

set -x

if [[ ! -d resty-cli ]]; then
    git clone \
        https://github.com/openresty/resty-cli \
        resty-cli
fi

cp -av ./resty-cli/t ./

for patch in ./tests/resty-cli-patches/*; do
    patch --verbose \
        -p0 \
        -i "$patch"
done
