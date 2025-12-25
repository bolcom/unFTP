---
title: Enabling tokio-console
---

You can use [tokio-console](https://github.com/tokio-rs/console) to analyze async tasks running in unFTP. To do this you
need to compile a build or run with the `tokio_console` feature enabled while also enabling the `tokio_unstable cfg`.

For example:

```sh
RUSTFLAGS="--cfg tokio_unstable" cargo build --features tokio_console
```

or:

```shell
RUSTFLAGS="--cfg tokio_unstable" cargo run --features tokio_console -- -vv --auth-type=anonymous
```

By default, unFTP will listen on `127.0.0.1:6669` for connections from tokio-console. You can customize this using the `--bind-address-tokio-console` flag:

```shell
RUSTFLAGS="--cfg tokio_unstable" cargo run --features tokio_console -- --bind-address-tokio-console 127.0.0.1:6670 --auth-type=anonymous
```

This allows multiple unFTP servers to run simultaneously on the same host, each with their own tokio-console instance.

