---
title: Installation
---

This page covers the various ways to install unFTP, including precompiled binaries, package repositories, building from source, and Docker images.

## Binaries

[Precompiled binaries for unFTP are available](https://github.com/bolcom/unFTP/releases) for these targets:

- `unftp_x86_64-unknown-linux-gnu` - Linux x86_64, dynamically linked with PAM support.
- `unftp_x86_64-unknown-linux-musl` - Linux x86_64, statically linked without PAM support.
- `unftp_aarch64-unknown-linux-musl` - Linux ARM64, statically linked without PAM support.
- `unftp_x86_64-apple-darwin` - macOS Intel.
- `unftp_aarch64-apple-darwin` - macOS Apple silicon.
- `unftp_x86_64-pc-windows-msvc.exe` - Windows x86_64.

Check the assets for the release you are using; older releases do not include the Linux ARM64 binary.

### To install with Curl:

Linux (static, no PAM):

```sh
curl -L https://github.com/bolcom/unFTP/releases/download/v0.15.2/unftp_x86_64-unknown-linux-musl \
  | sudo tee /usr/local/bin/unftp > /dev/null && sudo chmod +x /usr/local/bin/unftp
```

Linux (dynamic with PAM support):

```sh
curl -L https://github.com/bolcom/unFTP/releases/download/v0.15.2/unftp_x86_64-unknown-linux-gnu \
  | sudo tee /usr/local/bin/unftp > /dev/null && sudo chmod +x /usr/local/bin/unftp
```

macOS Intel:

```sh
curl -L https://github.com/bolcom/unFTP/releases/download/v0.15.2/unftp_x86_64-apple-darwin \
  | sudo tee /usr/local/bin/unftp > /dev/null && sudo chmod +x /usr/local/bin/unftp
```

macOS ARM:

```sh
curl -L https://github.com/bolcom/unFTP/releases/download/v0.15.2/unftp_aarch64-apple-darwin \
  | sudo tee /usr/local/bin/unftp > /dev/null && sudo chmod +x /usr/local/bin/unftp
```

### To install using package repositories

#### AUR

```sh
yay -S unftp
```

#### FreeBSD Ports

```sh
pkg install ftp/unftp
```

#### Nixpkgs

```sh
nix-shell -p uradvd
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

Now that we've covered installation, let's proceed to [starting unFTP](/server/starting).
