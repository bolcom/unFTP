---
title: unFTP
---

## When you need to FTP 📁, but don't want to... Deploy 🚀 an FTPS server to Kubernetes or build 🧰 your own.
---

**unFTP is an open-source FTP(S)** (not SFTP) server aimed at the **Cloud** that allows bespoke **extension** through 
its pluggable authenticator, storage back-end and user detail store architectures. It aims to bring features typically 
needed in cloud environments like integration with proxy servers, Prometheus monitoring and shipping of structured 
logs while capitalizing on the memory safety and speed provided by its implementation language, [Rust](https://www.rust-lang.org/).

unFTP is first an **embeddable library** ([libunftp](https://crates.io/crates/libunftp)) and second an 
FTPS **server application** ([unFTP](https://github.com/bolcom/unFTP)). You can run it out of the box, embed it in your 
app, craft your own server or build extensions for it.

**un**FTP tries to **un**tangle you from old-school environments so you can move all the things, even FTP, to the cloud 
while your users still get that familiar FTP feeling.

### Looking for something else?

- [libunftp on Github](https://github.com/bolcom/libunftp)
- [libunftp API docs](https://docs.rs/libunftp/latest)
- [libunftp crate](https://crates.io/crates/libunftp)
- [unFTP server on Github](https://github.com/bolcom/unFTP)
- [Storage Backends for libunftp](https://crates.io/search?page=1&per_page=10&q=unftp-sbe-)
- [Authentication Backends for libunftp](https://crates.io/search?page=1&per_page=10&q=unftp-auth-)
