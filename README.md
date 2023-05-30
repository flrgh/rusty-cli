# rusty-cli
[![test](https://github.com/flrgh/rusty-cli/actions/workflows/test.yml/badge.svg)](https://github.com/flrgh/rusty-cli/actions/workflows/test.yml)
[![resty-cli compat](https://github.com/flrgh/rusty-cli/actions/workflows/test-compat.yml/badge.svg)](https://github.com/flrgh/rusty-cli/actions/workflows/test-compat.yml)

[resty-cli](https://github.com/openresty/resty-cli), reimplemented

## Why does this exist?

resty-cli is a necessary component of many OpenResty installations, but it is
the bane of of many package maintainers due to its runtime dependency on Perl.

This project seeks to be a drop-in replacement for resty-cli with no runtime
dependencies (aside from OpenResty itself, of course), shippable as a single
binary. This translates to slimmer container images and fewer security foibles
to worry about when packaging OpenResty or other software that bundles it.

## Status

`rusty-cli` is working well enough for development usage. There have been
several times in the last few months where I symlinked it in place of `resty`
in order to test out a real world use case and simply forgot it was there,
leaving it in place for weeks at a time (I work on OpenResty-related stuff
pretty much every day).

It is also passing all of `resty-cli`'s tests. However, the OpenResty tests
are far from exhaustive, so I am not ready to call this production-ready until
I add some more integration tests myself.

## Features

Almost all of the core features of `resty-cli` have been implemented:

- NGINX config features
    - main/root
        - [x] error log level (`--errlog-level <level>`)
        - [x] includes (`--main-include <path>`)
        - [x] directives (`--main-conf <directive>`)
    - `events {}`
        - [x] worker_connections (`-c <num>`)
    - `http {}`
        - [x] includes (`--http-include <path>`)
        - [x] directives (`--http-conf <directive>`)
        - [x] shdict (`--shdict <name size>`)
    - `stream {}`
        - [x] enable/disable (`--no-stream`)
        - [x] directives (`--stream-conf <directive>`)
    - lua
        - [x] inline lua eval (`-e <expr>`)
        - [x] file execution via positional arg
        - [x] additional cli args passed to lua
        - [x] additional package path/cpath entries (`-I <dir>`)
        - [x] jit features (`-j v|dump|off`)
        - [x] require lib (`-l <lib>`)
    - `resolver`
        - [x] nameserver override (`--ns <addr>`)
        - [x] ipv6 enable/disable (`--resolve-ipv6`)
        - [x] parse /etc/resolve.conf for defaults
        - [x] fallback to Google DNS (`8.8.8.8`, `8.8.4.4`)
- execution customizations
    - [x] gdb (`--gdb` / `--gdb-opts <opts>`)
    - [x] override NGINX binary/path (`--nginx <path>`)
    - [x] stap (`--stap` / `--stap-opts <opts>`)
    - [x] valgrind (`--valgrind` / `--valgrind-opts <opts>`)
    - [x] Mozilla rr (`--rr`)
    - [x] generic user runner (`--user-runner <opts>`)
- cli flag commands
    - [x] help (`--help` | `-h`)
    - [x] version (`-V` | `-v`)
- runtime behavior
    - [x] trap+update+forward signals to NGINX process
    - [x] set exit status from NGINX
- platform/OS support
    - [x] *nix
    - [ ] Windows (this might be dropped as a goal altogether)

## Non-goals

While rusty-cli strives to be _functionally_ compatible with resty-cli where it
counts, there are some things that it does not care to replicate exactly:

* Temporary files
    * Example: `nginx.conf` and `.lua` file(s) generated from inline expressions
    * Disclaimer: formatting, whitespace, and [non-significant] ordering
      differences may occur.
* CLI metadata
    * Example: `--help` text, invalid cli arg error messages
    * Disclaimer: Anything that is intended for human eyes and not typically
      machine-parseable will not be byte-for-byte identical to resty-cli.

In many cases I _have_ replicated string outputs exactly as resty-cli, but only
because this makes compatibility testing easier for me (I need to maintain
patches for any of resty-cli's tests that produce different string output from
rusty-cli). Do not rely on this if you are using rusty-cli.

**If you are using resty-cli in a way that is sensitive to the exact contents of
CLI metadata, error messages, and nginx.conf, I recommend against using rusty-cli.**

## TODO

- [x] tests
    - [x] test against [resty-cli's test suite](https://github.com/openresty/resty-cli/tree/master/t)
    - [x] additional in-repo resty-cli compatibility tests
        - [x] custom runner arg parsing and execution
        - [x] lua `arg` global generation
        - [x] nginx.conf generation
- [ ] automated binary releases
    - [x] x86_64-unknown-linux-gnu
    - [ ] x86_64-unknown-linux-musl
    - [ ] aarch64-unknown-linux-gnu
    - [ ] aarch64-unknown-linux-musl
    - [ ] x86_64-apple-darwin
    - [ ] aarch64-apple-darwin

## Acknowledgements

Thanks to the [OpenResty](https://openresty.org/) folks for creating an awesome
piece of software that is fun to build with ❤️.

## License

This module is licensed under the BSD license.

Copyright (C) 2022-2023, by Michael Martin <flrgh@protonmail.com>.

All rights reserved.

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

* Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.

* Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
