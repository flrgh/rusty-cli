# rusty-cli

[resty-cli](https://github.com/openresty/resty-cli), written in rust

## Why does this exist?

resty-cli is a necessary component of many OpenResty installations, but it is
the bane of of many package maintainers due to its runtime dependency on Perl.

This project seeks to be a drop-in replacement for resty-cli with no runtime
dependencies (aside from OpenResty itself, of course), shippable as a single
binary. This translates to slimmer container images and fewer security foibles
to worry about when packaging OpenResty or other software that bundles it.

## Status

Many of the core features are implemented, but there's a ways to go to acheive
100% compatibility with resty-cli. Here's a rough outline of the current status:

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

### Non-goals

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

### TODO

- [ ] release tooling/automation
- [x] CI integration
- [ ] testing
    - [ ] in-repo unit/functional tests
    - [x] test against [resty-cli's test suite](https://github.com/openresty/resty-cli/tree/master/t)
