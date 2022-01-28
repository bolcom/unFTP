# unFTP

[![Crate Version](https://img.shields.io/crates/v/unftp.svg)](https://crates.io/crates/unftp)
[![Build Status](https://travis-ci.org/bolcom/unFTP.svg)](https://travis-ci.org/bolcom/unFTP) 
[![Docker Pulls](https://img.shields.io/docker/pulls/bolcom/unftp.svg?maxAge=2592000?style=plastic)](https://hub.docker.com/r/bolcom/unftp/)
[![Follow on Telegram](https://img.shields.io/badge/follow%20on-Telegram-brightgreen.svg)](https://t.me/unftp)

When you need to FTP, but don't want to.

![logo](logo.png)

unFTP is a FTP(S) server written in [Rust](https://www.rust-lang.org) and built on top of [libunftp](https://github.com/bolcom/libunftp) and the [Tokio](https://tokio.rs) asynchronous run-time. It is **un**like your normal FTP server in that it provides:

- Configurable Authentication (e.g. Anonymous, [PAM](https://en.wikipedia.org/wiki/Linux_PAM) or a JSON file).
- Configurable storage back-ends (e.g. [GCS](https://cloud.google.com/storage/) or filesystem)
- An HTTP server with health endpoints for use for example in Kubernetes for readiness and liveness probes.
- Integration with [Prometheus](https://prometheus.io) for monitoring.
- A proxy protocol mode for use behind proxies like HA Proxy and Nginx.
- Structured logging and the ability to ship logs to a Redis instance.

With unFTP, you can present RFC compliant FTP(S) to the outside world while freeing yourself to use modern APIs and 
techniques on the inside of your perimeter.

## Installation

### Binaries

[Precompiled binaries for unFTP are available](https://github.com/bolcom/unFTP/releases) for Linux and macOS. On Linux
you can choose between a statically linked image (no PAM integration) or a dynamically linked image with PAM integration:

- unftp_x86_64-apple-darwin - macOS
- unftp_x86_64-unknown-linux-musl - Linux statically linked, no PAM support.
- unftp_x86_64-unknown-linux-gnu - Dynamically linked with PAM support.

#### To install with Curl:

Linux (static, no PAM):

```sh
curl -L https://github.com/bolcom/unFTP/releases/download/v0.13.0/unftp_x86_64-unknown-linux-musl \
  | sudo tee /usr/local/bin/unftp > /dev/null && sudo chmod +x /usr/local/bin/unftp
```

Linux (dynamic with PAM support):

```sh
curl -L https://github.com/bolcom/unFTP/releases/download/v0.13.0/unftp_x86_64-unknown-linux-gnu \
  | sudo tee /usr/local/bin/unftp > /dev/null && sudo chmod +x /usr/local/bin/unftp
```

macOS:

```sh
curl -L https://github.com/bolcom/unFTP/releases/download/v0.13.0/unftp_x86_64-apple-darwin \
  | sudo tee /usr/local/bin/unftp > /dev/null && sudo chmod +x /usr/local/bin/unftp
```

### From Source

You'll need [Rust](https://rust-lang.org) 1.45 (including `cargo`) or higher to build unFTP. Then: 

```sh
cargo install unftp
```

and find unftp in `~/.cargo/bin/unftp`. You may want to add `~/.cargo/bin` to your PATH if you haven't done so. The above 
merely creates the binary there, it won't start it as a service at the moment.

#### Features

unFTP offers optional features in its Cargo.toml:

- `cloud_storage`: enables the Google Cloud Storage (GCS) storage backend
- `jsonfile_auth`: enables the JSON file authentication module
- `pam_auth`: enables the PAM authentication module
- `rest_auth`: enables the REST authentication module

## Usage

Both command line arguments and environment variables are available in unFTP. To show a list of available 
program arguments:

```sh
unftp --help
```

To run with default settings:

```sh
unftp
```

Example running an instance with a filesystem back-end and custom port:

```sh
unftp \
  --root-dir=/home/unftp/data \
  --bind-address=0.0.0.0:2121 \
  --passive-ports=50000-51000 \
  -vv
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

# Run, pointing to cert and key and require TLS on the control channel
unftp \
  --root-dir=/home/unftp/data \
  --ftps-certs-file=/home/unftp/unftp.crt \
  --ftps-key-file=/home/unftp/unftp.key \
  --ftps-required-on-control-channel=all
```

Enabling the [Prometheus](https://prometheus.io) exporter on (`http://../metrics`), binding to port 8080:

```sh
unftp \
  --bind-address=0.0.0.0:2121 \
  --bind-address-http=0.0.0.0:8080 \
  --root-dir=/home/unftp/data
```

Run with the GCS (Google Cloud Storage) back-end:

```sh
unftp \
  --sbe-type=gcs \
  --sbe-gcs-bucket=mybucket \
  --sbe-gcs-key-file=file
```

Run behind a proxy in [proxy protocol](https://www.haproxy.com/blog/haproxy/proxy-protocol/) mode:

```sh
unftp \
    --proxy-external-control-port=2121
```

Run sending logs to a Redis list:

```sh
unftp \
    --log-redis-host=localhost \
    --log-redis-key=logs-key \
    --log-redis-port=6379
```

Authenticate with credentials stored in a JSON file:

Create a credentials file (e.g. credentials.json):

```json
[
  {
    "username": "alice",
    "password": "12345678"
  },
  {
    "username": "bob",
    "password": "secret"
  }
]
```

```sh
unftp \
    --auth-type=json \
    --auth-json-path=credentials.json
```

Require Mutual TLS:

```sh
# Create Server Root Key and Certificate
openssl genrsa -out unftp_client_ca.key 2048
openssl req -new -x509 -days 365 \
	-key unftp_client_ca.key \
        -subj '/CN=unftp-ca.mysite.com/O=bol.com/C=NL' \
	-out unftp_client_ca.crt

# Create a client side key
openssl genrsa -out client.key 2048

# Create a client side certificate signing request (CSR)
openssl req -new -sha256 \
    -key client.key \
    -subj '/CN=unftp-client.mysite.com/O=bol.com/C=NL' \
    -reqexts SAN \
    -config <(cat /etc/ssl/openssl.cnf \
        <(printf "\n[SAN]\nsubjectAltName=DNS:localhost")) \
    -out client.csr

# Sign the certificate with our own CA
openssl x509 -req \
  -in client.csr \
  -CA unftp_client_ca.crt \
  -CAkey unftp_client_ca.key \
  -CAcreateserial \
  -extfile <(printf "subjectAltName=DNS:localhost") \
  -out client.crt \
  -days 1024 \
  -sha256
  
# Run unFTP pointing to the CA cert
unftp \
  --root-dir=/home/unftp/data \
  --ftps-certs-file=/home/unftp/unftp.crt \
  --ftps-key-file=/home/unftp/unftp.key \
  --ftps-required-on-control-channel=all \
  --ftps-client-auth=require \
  --ftps-trust-store=/Users/xxx/unftp/unftp_client_ca.crt
  
# From another terminal: Connect with CURL, sending the client certificate
curl -v \
  --insecure \
  --user 'test:test' \
  --ftp-ssl --ssl-reqd \
  --ftp-pasv --disable-epsv \
  --cacert unftp_client_ca.crt \
  --cert client.crt \
  --key client.key \
  --cert-type PEM \
  --pass '' \
  --tlsv1.2 \
  ftp://localhost:2121/  
```

To do per-user settings you can expand the above mentioned JSON file to also include some per user settings:

```json
[
  {
    "username": "alice",
    "password": "12345678",
    "vfs_perms": ["-mkdir","-rmdir","-del","-ren", "-md5"],
    "root": "alice",
    "account_enabled": true
  },
  {
    "username": "bob",
    "password": "secret",
    "client_cert": {
      "allowed_cn": "bob-the-builder"
    }
  },
  {
    "username": "vincent",
    "root": "vincent",
    "vfs_perms": ["none", "+put", "+md5"],
    "client_cert": {}
  }  
]
```

And let unFTP point to it:

```sh
unftp \
    --auth-type=json \
    --auth-json-path=users.json \
    --usr-json-path=users.json \
    ...
```

In the above configuration we use:

* `vfs_perms` - Specifies what permissions users can have. Alice cannot create directories, remove them, delete files nor 
  calculate the md5 of files. Bob can do everything while Vincent can only do uploads and calculate md5 files. Valid values
  here are "none", "all", "-mkdir, "-rmdir, "-del","-ren", "-md5", "-get", "-put", "-list", "+mkdir", "+rmdir", "+del", 
  "+ren", "+md5", "+get", "+put" and "+list".
* `root` - Sets the home directory of the user relative to the storage back-end root. Alice can only see files inside 
  `$SB_ROOT/alice`, Bob can see all files and Vincent thinks `$SB_ROOT/vincent` is the FTP root similar to Alice.
* `account_enabled` - Allows to disable the user's account completely
* `client_cert` - Allows specifying whether a client certificate is required and how to handle it. Alice logs in with 
  normal user/password authentication. No client certificate needed. Bob needs to provide a valid client certificate 
  with common name (CN) containing, 'bob-the-builder' and also needs to provide a password. Vincent can do passwordless 
  login when providing a valid certificate.


## Docker image

The project contains templated Dockerfiles . To get a list of available commands, run:

```sh
make help
```

We offer 3 different options for building an unFTP docker image:

- `scratch`: builds the binary in [rust:slim](https://hub.docker.com/_/rust) and deploys in a `FROM scratch` image. The unFTP binary is statically linked using [musl libc](https://www.musl-libc.org/).
- `alpine` (default): builds in [rust:slim](https://hub.docker.com/_/rust) and deploy in alpine. This image is built with musl instead of a full-blown libc. The unFTP binary is statically linked using [musl libc](https://www.musl-libc.org/).
- `alpine-debug`: same images as `alpine` but using the debug build of unftp and adds tools like [lftp](https://lftp.yar.ru/) and [CurlFtpFS](http://curlftpfs.sourceforge.net/) while also running as root.
- `alpine-istio`: same as `alpine` but with [scuttle](https://github.com/redboxllc/scuttle) installed. For use together with [Istio](https://istio.io/).
- `alpine-istio-debug`: same as alpine-debug but with the additions of `alpine-istio`.  

To build the alpine docker image:

```sh
make docker-image-alpine
```

Alternatively you can download pre-made images from docker hub e.g.:

```sh
docker pull bolcom/unftp:v0.13.0-alpine
docker pull bolcom/unftp:v0.13.0-alpine-istio
docker pull bolcom/unftp:v0.13.0-scratch
```

Example running it:

```sh
docker run \
  -e ROOT_DIR=/ \
  -e UNFTP_LOG_LEVEL=info \
  -e UNFTP_FTPS_CERTS_FILE='/unftp.crt' \
  -e UNFTP_FTPS_KEY_FILE='/unftp.key' \
  -e UNFTP_PASSIVE_PORTS=50000-50005 \
  -e UNFTP_SBE_TYPE=gcs \
  -e UNFTP_SBE_GCS_BUCKET=the-bucket-name \
  -e UNFTP_SBE_GCS_KEY_FILE=/key.json \
  -p 2121:2121 \
  -p 50000:50000 \
  -p 50001:50001 \
  -p 50002:50002 \
  -p 50003:50003 \
  -p 50004:50004 \
  -p 50005:50005 \
  -p 8080:8080 \
  -v /Users/xxx/unftp/unftp.key:/unftp.key  \
  -v /Users/xxx/unftp/unftp.crt:/unftp.crt \
  -v /Users/xxx/unftp/the-key.json:/key.json \
  -ti \
  bolcom/unftp:v0.13.0-alpine
```

## Getting help and staying informed

Support is given on a best effort basis. You are welcome to engage us on [the discussions page](https://github.com/bolcom/unftp/discussions)
or create a Github issue.

You can also follow news and talk to us on [Telegram](https://t.me/unftp) 

## License

You're free to use, modify and distribute this software under the terms of the Apache-2.0 license.

## See also

- [libunftp](https://github.com/bolcom/libunftp)
