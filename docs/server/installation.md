---
title: Installation
---

## Binaries

[Precompiled binaries for unFTP are available](https://github.com/bolcom/unFTP/releases) for Linux and macOS. On Linux
you can choose between a statically linked image (no PAM integration) or a dynamically linked image with PAM
integration:

- unftp_x86_64-apple-darwin - macOS
- unftp_x86_64-unknown-linux-musl - Linux statically linked, no PAM support.
- unftp_x86_64-unknown-linux-gnu - Dynamically linked with PAM support.

### To install with Curl:

Linux (static, no PAM):

```sh
curl -L https://github.com/bolcom/unFTP/releases/download/v0.14.6/unftp_x86_64-unknown-linux-musl \
  | sudo tee /usr/local/bin/unftp > /dev/null && sudo chmod +x /usr/local/bin/unftp
```

Linux (dynamic with PAM support):

```sh
curl -L https://github.com/bolcom/unFTP/releases/download/v0.14.6/unftp_x86_64-unknown-linux-gnu \
  | sudo tee /usr/local/bin/unftp > /dev/null && sudo chmod +x /usr/local/bin/unftp
```

macOS Intel:

```sh
curl -L https://github.com/bolcom/unFTP/releases/download/v0.14.6/unftp_x86_64-apple-darwin \
  | sudo tee /usr/local/bin/unftp > /dev/null && sudo chmod +x /usr/local/bin/unftp
```

macOS ARM:

```sh
curl -L https://github.com/bolcom/unFTP/releases/download/v4/unftp_aarch64-apple-darwin \
  | sudo tee /usr/local/bin/unftp > /dev/null && sudo chmod +x /usr/local/bin/unftp
```

## From Source

You'll need [Rust](https://rust-lang.org) 1.67.1 (including `cargo`) or higher to build unFTP. Then:

```sh
cargo install unftp
```

and find unftp in `~/.cargo/bin/unftp`. You may want to add `~/.cargo/bin` to your PATH if you haven't done so. The
above
merely creates the binary there, it won't start it as a service at the moment.

## Docker Images

See [the Docker section](/server/docker)
