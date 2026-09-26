# Project basics

- unFTP is a Rust FTP/FTPS server designed for cloud deployments, built on libunftp and Tokio.
- This repository contains the server application, including configuration and integrations for storage, authentication, monitoring, logging, and notifications.
- Public website and user documentation: https://unftp.rs
- GitHub project: https://github.com/bolcom/unFTP
- Closely related library project: https://github.com/bolcom/libunftp
- Many extensions used by unFTP can be found on crates.io by searching for `unftp-`: https://crates.io/search?q=unftp-
- libunftp provides the underlying FTP/FTPS server library; this application configures and integrates it.
- Application code lives in `src/`; website documentation lives in `docs/`.
