# unFTP

[![Build Status](https://travis-ci.org/bolcom/unFTP.svg)](https://travis-ci.org/bolcom/unFTP)

When you need to FTP, but don't want to.

With unFTP, you can present FTP to the outside world while freeing yourself to use all the modern APIs and techniques you want to.
By storing everything in Google Buckets and authenticating against an external service it requires no local state.

unFTP is currently very much under development and totally not usable yet.

## Prerequisites

You'll need [Rust](https://rust-lang.org) 1.31 (including `cargo`) or higher to build unFTP.
There are no runtime dependencies besides the OS and libc.

## Docker

To build a docker image based on alpine:

```sh
make docker-alpine
```

To build and run unFTP inside an alpine docker image:

```sh
make docker-run-alpine
```

Partly as an example, there is also 'minimal' image available that is statically linked and build `FROM scratch`. To use it use `make docker-minimal` and `make docker-run-minimal`.
For the full list of supplied docker images, use `make docker-list`.

## License

You're free to use, modify and distribute this software under the terms of the Apache-2.0 license, whichever has your preference.
