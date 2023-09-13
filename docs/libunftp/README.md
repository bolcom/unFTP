---
title: The Library
---

[![Crate Version](https://img.shields.io/crates/v/libunftp.svg)](https://crates.io/crates/libunftp)
[![API Docs](https://docs.rs/libunftp/badge.svg)](https://docs.rs/libunftp)
[![Build Status](https://travis-ci.org/bolcom/libunftp.svg)](https://travis-ci.org/bolcom/libunftp)
[![Crate License](https://img.shields.io/crates/l/libunftp.svg)](https://crates.io/crates/libunftp)
[![Follow on Telegram](https://img.shields.io/badge/Follow%20on-Telegram-brightgreen.svg)](https://t.me/unftp)

# libunftp - The FTPS library

**libunftp** is a [Rust](https://www.rust-lang.org/) [crate](https://crates.io/crates/libunftp) that you can use to 
build your own FTPS server with. You can extend it with your own storage back-ends or authentication back-ends.

It runs on top of the [Tokio](https://tokio.rs) asynchronous run-time and tries to make use of Async IO as much as
possible.

Feature highlights:

* 39 Supported FTP commands and growing
* Ability to implement own storage back-ends
* Ability to implement own authentication back-ends
* Explicit FTPS (TLS)
* Mutual TLS (Client certificates)
* TLS session resumption
* Prometheus integration
* Structured Logging
* [Proxy Protocol](https://www.haproxy.com/blog/haproxy/proxy-protocol/) support
* Automatic session timeouts
* Per user IP allow lists

Known storage back-ends:

* [unftp-sbe-fs](https://crates.io/crates/unftp-sbe-fs) - Stores files on the local filesystem
* [unftp-sbe-gcs](https://crates.io/crates/unftp-sbe-gcs) - Stores files in Google Cloud Storage
* [unftp-sbe-rooter](https://crates.io/crates/unftp-sbe-rooter) - Wraps another storage back-end in order to root a user to a specific home directory.
* [unftp-sbe-restrict](https://crates.io/crates/unftp-sbe-rooter) - Wraps another storage back-end in order to restrict the FTP operations a user can do i.e. provide authorization.

Known authentication back-ends:

* [unftp-auth-jsonfile](https://crates.io/crates/unftp-auth-jsonfile) - Authenticates against JSON text.
* [unftp-auth-pam](https://crates.io/crates/unftp-auth-pam) - Authenticates via [PAM](https://en.wikipedia.org/wiki/Linux_PAM).
* [unftp-auth-rest](https://crates.io/crates/unftp-auth-rest) - Consumes an HTTP API to authenticate.

See the [github page](https://github.com/bolcom/libunftp) or the [API Documentation](https://docs.rs/libunftp) for more details.
