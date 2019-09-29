# unFTP

[![Build Status](https://travis-ci.org/bolcom/unFTP.svg)](https://travis-ci.org/bolcom/unFTP)

When you need to FTP, but don't want to.

![logo](logo.png)

unFTP is a FTP server written in [Rust](https://www.rust-lang.org) and built on top of [libunftp](https://github.com/bolcom/libunftp) and the [Tokio ](https://tokio.rs) asynchronous run-time. It is **un**like your normal FTP server in that it provides:

- Configurable Authentication (e.g. Anonymous, [PAM](https://en.wikipedia.org/wiki/Linux_PAM) or a REST service).
- Configurable storage back-ends (e.g. [GCS](https://cloud.google.com/storage/) or filesystem)
- Integration with [Prometheus](https://prometheus.io) for monitoring.

With unFTP, you can present RFC compliant FTP to the outside world while freeing yourself to use modern APIs and techniques on the inside of your perimeter.

**unFTP is in its early development stages and therefore not suitable for use in production yet.**

## Prerequisites

You'll need [Rust](https://rust-lang.org) 1.37 (including `cargo`) or higher to build unFTP.
There are no runtime dependencies besides the OS and libc.

Run `make help` to see an overview of the supplied *make* targets.

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

Enabling the [Prometheus](https://prometheus.io) exporter, binding to port 8080:

```sh
cargo run -- \
  --bind-address=0.0.0.0:2121 \
  --bind-address-http=0.0.0.0:8080 \
  --home-dir=/home/unftp/data
```

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

## License

You're free to use, modify and distribute this software under the terms of the Apache-2.0 license.

## See also

- [libunftp](https://github.com/bolcom/libunftp)
