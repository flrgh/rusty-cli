# rusty-cli

[![test](https://github.com/flrgh/rusty-cli/actions/workflows/test.yml/badge.svg)](https://github.com/flrgh/rusty-cli/actions/workflows/test.yml)
[![resty-cli compat](https://github.com/flrgh/rusty-cli/actions/workflows/test-compat.yml/badge.svg)](https://github.com/flrgh/rusty-cli/actions/workflows/test-compat.yml)

[resty-cli](https://github.com/openresty/resty-cli), reimplemented

## Why does this exist?

`resty-cli` is heavily relied upon in most OpenResty deployments, but its runtime
dependency on Perl often makes it less than ideal for package maintainers
and operators.

This project is a drop-in replacement for `resty-cli` with no runtime
dependencies (aside from OpenResty itself, of course), shippable as a single
binary. This translates to slimmer container images and fewer security worries
when packaging OpenResty or other software that bundles it.

## Status

All features of `resty-cli` have been implemented.

`rusty-cli` passes all of `resty-cli`'s
[tests](https://github.com/openresty/resty-cli/tree/3022948ef3d670b915bcf7027bcdd917591b96e4/t)
in CI, and I've added some [additional tests](https://github.com/flrgh/rusty-cli/blob/fdbcda180830534dcc2a32c4f6901a927e6bf8f0/.github/workflows/test-compat.yml#L169-L176)
of my own to validate behavioral parity.

I have been using `rusty-cli` in place of `resty-cli` for almost a year now and
have yet to encounter any problems with it in my day-to-day.

## Compatibility Limitations

While `rusty-cli` strives to be _functionally_ compatible with `resty-cli` where
it counts, there are some things that it does not care to replicate exactly:

* Temporary files
    * **Example:** `nginx.conf` and `.lua` file(s) generated from inline expressions
    * **Disclaimer:** formatting, whitespace, and [non-significant] ordering
      differences may occur.
* CLI metadata
    * **Example:** `--help` text, invalid cli arg error messages
    * **Disclaimer:** Anything that is intended for human eyes and not typically
      machine-parseable will not be byte-for-byte identical to resty-cli.

In many cases I _have_ replicated string outputs exactly as `resty-cli`, but only
because this makes compatibility testing easier for me (I need to maintain
patches for any of `resty-cli`'s tests that produce different string output from
rusty-cli). Do not rely on this if you are using `rusty-cli`.

**If you are using resty-cli in a way that is sensitive to the exact contents of
CLI metadata, error messages, and nginx.conf, I recommend against using `rusty-cli`.**

## Non-Goals

* **Windows Support**: not planned at this time

## Compile-Time Environment Variables

Package maintainers can utilize some environment variables at build time to
customize behavior.

### Usage

```sh
MY_VAR=value cargo build
```

**Warning**: Cargo does not reliably invalidate build cache when setting
build-time env vars via `--config 'env.VAR="value"'`. For the most consistent
behavior, set environment vars via your shell.

### NGINX_PATH

This sets the default path to the `nginx` binary that `rusty-cli` will use
when not explicitly set via the `--nginx` command line option.

#### Example

```sh
# set a custom default nginx path
NGINX_PATH=/path/to/sbin/nginx cargo build

# prints `/path/to/sbin/nginx`
./path/to/rusty-cli -e 'os.execute("realpath /proc/" .. ngx.worker.pid() .. "/exe")'
```

#### Explanation

As a standalone tool, `resty-cli` checks a couple common locations for the
`nginx` binary before falling path to `$PATH` resolution ([source](https://github.com/openresty/resty-cli/blob/3022948ef3d670b915bcf7027bcdd917591b96e4/bin/resty#L487-L520)):

  1. `<bin>/../../nginx/sbin/nginx`
  2. `<bin>/../nginx`

Standard releases of `rusty-cli` replicate this behavior.

When `resty-cli` is installed as part of an official OpenResty package, it is
patched with the hardcoded nginx path at build time ([source](https://github.com/openresty/openresty/blob/9c9495b6f9277018e683bbee42ce2f6a0edf248d/util/configure#L1174-L1192)).

Compiling with `NGINX_PATH` enables parity with the OpenResty-bundled version of
`resty-cli`.

## TODO

- [x] tests
    - [x] test against [resty-cli's test suite](https://github.com/openresty/resty-cli/tree/master/t)
    - [x] additional in-repo resty-cli compatibility tests
        - [x] custom runner arg parsing and execution
        - [x] lua `arg` global generation
        - [x] nginx.conf generation
        - [x] nginx binary filesystem location search
- [ ] automated binary releases
    - [x] x86_64-unknown-linux-gnu
    - [x] x86_64-unknown-linux-musl
    - [x] aarch64-unknown-linux-gnu*
    - [x] aarch64-unknown-linux-musl*
    - [x] x86_64-apple-darwin*
    - [x] aarch64-apple-darwin*


\* These are built with new releases but not yet fully tested

## Acknowledgements

Thanks to the [OpenResty](https://openresty.org/) folks for creating an awesome
piece of software that is fun to build with ❤️.
