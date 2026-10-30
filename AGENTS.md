# Project basics

- unFTP is a Rust FTP/FTPS server designed for cloud deployments, built on libunftp and Tokio.
- This repository contains the server application, including configuration and integrations for storage, authentication, monitoring, logging, and notifications.
- GitHub project: https://github.com/Cloudinvoke/unFTP
- Maintained under the CloudInvoke project by Hannes de Jager and Rob klein Gunnewiek (https://github.com/robklg). unFTP is no longer affiliated with bol.com.
- The source for libunftp sits here: https://github.com/bolcom/libunftp
- Many extensions used by unFTP can be found on crates.io by searching for `unftp-`: https://crates.io/search?q=unftp-
- libunftp provides the underlying FTP/FTPS server library; this application configures and integrates it.
- Application code lives in `src/`
- The project tries to adhere loosly to a domain centric architecture layout like the Clean and Onion Architectures.
- Github Actions are used to build tests and run images

## Packaging

- We offer binaries that are hosted on Github
- We have a couple of docker images. There are *.Dockerfile.template files for these in the root. They build and publish via Github actions
- Images are still published to Docker Hub as `bolcom/unftp` until a CloudInvoke namespace is set up

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
- Don't add attribution to AI engines

## Pull requests

- Don't add attribution to AI engines

## Upgrading the Rust version

Ask if the Rust version should be upgraded before a release.

Do not upgrade the user's local compiler; remind them to run
`rustup update stable` if they are behind.

Keep these pins in lockstep:

- `.github/workflows/rust.yml` (`env.RUST_VERSION`)
- `Makefile` (`RUST_VERSION`). This is substituted into
  `*.Dockerfile.template` when generating the gitignored root
  `*.Dockerfile` files.
- `packaging/docker/*.Dockerfile.ci` (`FROM rust:<version>-slim`),
  which GitHub Actions uses to publish images.

Also note the new version under Unreleased in `CHANGELOG.md`.

`docs/server/installation.md` documents a *minimum* Rust version
for building from source, not the CI/Docker toolchain. Only bump
that if the project actually requires a newer MSRV.
