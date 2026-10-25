# Project basics

- unFTP is a Rust FTP/FTPS server designed for cloud deployments, built on libunftp and Tokio.
- This repository contains the server application, including configuration and integrations for storage, authentication, monitoring, logging, and notifications.
- GitHub project: https://github.com/bolcom/unFTP
- The source for libunftp sits here: https://github.com/bolcom/libunftp
- Many extensions used by unFTP can be found on crates.io by searching for `unftp-`: https://crates.io/search?q=unftp-
- libunftp provides the underlying FTP/FTPS server library; this application configures and integrates it.
- Application code lives in `src/`
- The project tries to adhere loosly to a domain centric architecture layout like the Clean and Onion Architectures.
- Github Actions are used to build tests and run images

## Packaging

- We offer binaries that are hosted on Github
- We have a couple of docker images. There are *.Dockerfile.template files for these in the root. They build and publish via Github actions

## Website

- Public website and user documentation: https://unftp.rs
- Doctave (https://github.com/Doctave/doctave) is currently used to build the website
- Website documentation lives in `docs/` and is done in markdown.

## Commit messages

- No conventional commits please
- Separate subject from body with a blank line
- Limit the subject line to roughly 50 characters
- Capitalize the subject line
- Do not end the subject line with a period
- Use the imperative mood in the subject line
- Wrap the body at 72 characters
- Use the body to explain what and why and context vs. how
- Write thoughtful commit messages like David Tolnay
- Write for the next person reading git log: A good commit title should let you scan history quickly without needing to run git show for every minor bump
