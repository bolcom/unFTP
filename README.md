# unFTP

[![Build Status](https://github.com/bolcom/unftp/workflows/CI/badge.svg)](https://github.com/bolcom/unftp/actions)

When you need to FTP, but don't want to.

![logo](logo.png)

unFTP is a FTP server written in [Rust](https://www.rust-lang.org) and built on top of [libunftp](https://github.com/bolcom/libunftp) and the [Tokio](https://tokio.rs) asynchronous run-time. It is **un**like your normal FTP server in that it provides:

- Configurable Authentication (e.g. Anonymous, [PAM](https://en.wikipedia.org/wiki/Linux_PAM) or a REST service).
- Configurable storage back-ends (e.g. [GCS](https://cloud.google.com/storage/) or filesystem)
- Integration with [Prometheus](https://prometheus.io) for monitoring.

With unFTP, you can present RFC compliant FTP to the outside world while freeing yourself to use modern APIs and techniques on the inside of your perimeter.

**unFTP is in its early development stages and therefore not suitable for use in production yet.**

## Prerequisites

You'll need [Rust](https://rust-lang.org) 1.40 (including `cargo`) or higher to build unFTP.
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
  --root-dir=/home/unftp/data \
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
  --root-dir=/home/unftp/data \
  --ftps-certs-file=/home/unftp/unftp.crt \
  --ftps-key-file=/home/unftp/unftp.key
```

Enabling the [Prometheus](https://prometheus.io) exporter, binding to port 8080:

```sh
cargo run -- \
  --bind-address=0.0.0.0:2121 \
  --bind-address-http=0.0.0.0:8080 \
  --root-dir=/home/unftp/data
```

With the GCS back-end:

```sh
cargo run -- \
  --sbe-type=gcs \
  --sbe-gcs-bucket=mybucket \
  --sbe-gcs-key-file=file
```

## Docker

Dockerfile is templated. To get a list of available commands, run:

```sh
make
```

We offer 3 different options for building an unFTP docker image:

- `minimal`: an empty image containing a static build of unFTP. *WARNING*: this is broken right now, as Cargo can only compile static binary if all the dependent libraries is also statically built.
- `alpine` (default): build unftp in rust-slim and deploy in alpine. This image is built with musl instead of a full-blown libc. Resulting image is about 20MB.
- `full`: build & run on the rust-slim base. Resulting image is over 1GB.

To build the default docker image:

```sh
make docker-image
```

To build and run unFTP inside the default docker image in the foreground:

```sh
make docker-run
```

## Features

unFTP offers optional features in its Cargo.toml:

- `pam`: enables the PAM authentication module
- `rest`: enables the REST authentication module
- `cloud_storage`: enables the Google Cloud Storage (GCS) storage backend

### Rest authentication

When enabled this feature allows authentication against a remote REST service.

It's a very generic REST client that fetches the endpoint specified in `--auth-rest-url` with the method specified in `--auth-rest-method`. *WARNING*: https is not yet supported.

If necessary, you can specify a request body in `--auth-rest-body`.

The special placeholders `{USER}` and `{PASS}` are replaced by the (clear-text) credentials provided by the user in the request url and body.

The response body is parsed as JSON. From here, a [JSON Pointer (RFC6901)](https://tools.ietf.org/html/rfc6901) compliant selector specified via `--auth-rest-selector` can be used to extract a value. For more info and examples, consult `serde_json`'s [manual](https://docs.serde.rs/serde_json/value/enum.Value.html#method.pointer).

The extracted value is matched against a regular expression specified via `--auth-rest-regex`. See [their documentation](https://crates.io/crates/regex) for more details.

If the regular expression matches, the user is authenticated. In any other case (lookup failure, timeout, non-matching regex) the authentication is refused.

## License

You're free to use, modify and distribute this software under the terms of the Apache-2.0 license.

## See also

- [libunftp](https://github.com/bolcom/libunftp)
