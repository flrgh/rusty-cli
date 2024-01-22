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

## TODO

- [x] tests
    - [x] test against [resty-cli's test suite](https://github.com/openresty/resty-cli/tree/master/t)
    - [x] additional in-repo resty-cli compatibility tests
        - [x] custom runner arg parsing and execution
        - [x] lua `arg` global generation
        - [x] nginx.conf generation
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
