# unFTP

[![Build Status](https://travis-ci.org/bolcom/unFTP.svg)](https://travis-ci.org/bolcom/unFTP)

When you need to FTP, but don't want to.

![logo](logo512.png)

With unFTP, you can present FTP to the outside world while freeing yourself to use all the modern APIs and techniques you want to.
By storing everything in Google Buckets and authenticating against an external service it requires no local state.

**unFTP is currently very much under development and totally not usable yet.**

## Prerequisites

You'll need [Rust](https://rust-lang.org) 1.37 (including `cargo`) or higher to build unFTP.
There are no runtime dependencies besides the OS and libc.

Run `make help` to see an overview of the supplied *make* targets.

## Docker

To build the default docker image:

```sh
make docker-image
```

To build and run unFTP inside the default docker image in the foreground:

```sh
make docker-run
```

Partly as an example, there is also 'minimal' image available that is statically linked and build `FROM scratch`. To use it use `make docker-minimal` and `make docker-run-minimal`.
For the full list of supplied docker images, use `make docker-list`.

## Running

To run with default settings:

```sh
cargo run
```

To show a list of program arguments:

```sh
cargo run -- \
  --help
```

Example running an instance with a filesystem back-end and custom port

```sh
cargo run -- \
  --home-dir=/home/unftp/data \
  --bind-address=0.0.0.0:2100
```

With FTPS enabled:

```sh
# Generate keypair
openssl req \
   -x509 \
   -newkey rsa:2048 \
   -nodes \
   -keyout unftp.key \
   -out unftp.crt \
   -days 3650 \
   -subj '/CN=www.myunftp.domain/O=My Company Name LTD./C=NL'

# Run, pointing to cert and key
cargo run -- \
  --home-dir=/home/unftp/data \
  --ftps-certs-file=/home/unftp/server.pem \
  --ftps-key-file=/home/unftp/server.key
```

## License

You're free to use, modify and distribute this software under the terms of the Apache-2.0 license.
