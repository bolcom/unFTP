---
title: The Library
---

[![Crate Version](https://img.shields.io/crates/v/libunftp.svg)](https://crates.io/crates/libunftp)
[![API Docs](https://docs.rs/libunftp/badge.svg)](https://docs.rs/libunftp)
[![Follow on Telegram](https://img.shields.io/badge/Follow%20on-Telegram-brightgreen.svg)](https://t.me/unftp)

# libunftp - The FTPS library

**libunftp** is a [Rust](https://www.rust-lang.org/) [crate](https://crates.io/crates/libunftp) that you can use to
build your own FTPS server with. You can extend it with your own storage back-ends or authentication back-ends.

It runs on top of the [Tokio](https://tokio.rs) asynchronous run-time and tries to make use of Async IO as much as
possible.

## Feature highlights

* 41 Supported FTP commands (see commands directory) and growing
* Ability to implement own storage back-ends
* Ability to implement own authentication back-ends
* Explicit FTPS (TLS)
* Mutual TLS (Client certificates)
* TLS session resumption
* Prometheus integration (enabled by default, can be disabled)
* Structured Logging
* [Proxy Protocol](https://www.haproxy.com/blog/haproxy/proxy-protocol/) support (enabled by default, can be disabled)
* Automatic session timeouts
* Per user IP allow lists
* Configurable cryptographic providers (ring or aws-lc-rs)

## Known Storage back-ends

| Crate | Description |
|-------|-------------|
| [unftp-sbe-fatfs](https://crates.io/crates/unftp-sbe-fatfs) | Provides read-only access to FAT filesystem images |
| [unftp-sbe-fs](https://crates.io/crates/unftp-sbe-fs) | Stores files on the local filesystem |
| [unftp-sbe-gcs](https://crates.io/crates/unftp-sbe-gcs) | Stores files in Google Cloud Storage |
| [unftp-sbe-iso](https://crates.io/crates/unftp-sbe-iso) | Provides FTP access to ISO 9660 files |
| [unftp-sbe-opendal](https://crates.io/crates/unftp-sbe-opendal) | Provides access to various storage services through Apache OpenDAL (supports S3, Azure Blob Storage, and more) |
| [unftp-sbe-restrict](https://crates.io/crates/unftp-sbe-restrict) | Wraps another storage back-end in order to restrict the FTP operations a user can do i.e. provide authorization |
| [unftp-sbe-rooter](https://crates.io/crates/unftp-sbe-rooter) | Wraps another storage back-end in order to root a user to a specific home directory |
| [unftp-sbe-webdav](https://crates.io/crates/unftp-sbe-webdav) | A WebDAV storage back-end providing translation between FTP & WebDAV |

See the [full list of storage back-ends on crates.io](https://crates.io/search?q=unftp-sbe) or the [Contribution Guide](https://github.com/bolcom/libunftp/blob/master/CONTRIBUTING.md#developing-your-own-authentication-or-storage-back-end-implementation) and [API Documentation](https://docs.rs/libunftp) if you want to create your own.

## Known Authentication back-ends

| Crate | Description |
|-------|-------------|
| [unftp-auth-jsonfile](https://crates.io/crates/unftp-auth-jsonfile) | Authenticates against JSON text |
| [unftp-auth-pam](https://crates.io/crates/unftp-auth-pam) | Authenticates via PAM |
| [unftp-auth-rest](https://crates.io/crates/unftp-auth-rest) | Consumes an HTTP API to authenticate |

See the [full list of authentication back-ends on crates.io](https://crates.io/search?q=unftp-auth) or the [Contribution Guide](https://github.com/bolcom/libunftp/blob/master/CONTRIBUTING.md#developing-your-own-authentication-or-storage-back-end-implementation) and [API Documentation](https://docs.rs/libunftp) if you want to create your own.

## Additional resources

See the [github page](https://github.com/bolcom/libunftp) or the [API Documentation](https://docs.rs/libunftp) for more details.
