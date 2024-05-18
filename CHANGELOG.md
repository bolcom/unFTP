# Changelog

## 2023-12-24 unftp v0.14.6

- Upgraded to libunftp v0.20.0
- Upgraded other dependencies
- Compile with Rust 1.78.0

## 2023-12-24 unftp v0.14.5

- Added support for source IP in REST authentication requests
- Upgraded to the latest version of libunftp
- Upgraded other dependencies

## 2023-09-17 unftp v0.14.4

- Upgraded to latest version of libunftp
- Upgraded other dependencies
- Fixed RUSTSEC-2020-0071
- Moved the rooter virtual file system to its [own crate](https://crates.io/crates/unftp-sbe-rooter).
- Moved the restricting virtual file system to its [own crate](https://crates.io/crates/unftp-sbe-restrict).

## 2023-06-16 unftp v0.14.3

- Upgraded to latest version of the GCS back-end

## 2023-06-02 unftp v0.14.2

- [#151](https://github.com/bolcom/unFTP/pull/151) Restart when receiving the HUP signal
- Fixed Windows build
- Upgraded dependencies including all crates from https://github.com/bolcom/libunftp
- Upgraded to Rust 1.70.0

## 2023-02-01 unftp v0.14.1

- Upgraded dependencies including all crates from https://github.com/bolcom/libunftp
  The main change here is [caching of access tokens in the GCS backend](https://github.com/bolcom/libunftp/issues/384)

## 2022-12-08 unftp v0.14.0

- The JSON authentication method (`--auth-json-path`) JSON user file (`--usr-json-path`) now support gzipped or
  gzipped+base64-encoded gzip files.
  The compression makes it possible to fit large configuration files in a Kubernetes configmap for example.
- Upgraded to unftp-auth-jsonfile v0.3.0 to support the gzipped or gzipped+base64-encoded auth json

## 2022-09-25 unftp v0.13.4

- Fixes from [libunftp v0.18.7](https://github.com/bolcom/libunftp/releases/tag/libunftp-0.18.7)
- Upgraded dependencies
- Upgraded to Rust 1.65.0

## 2022-09-25 unftp v0.13.3

- [#126](https://github.com/bolcom/unFTP/issues/126) Now support Elliptic Curve Private keys
- [#130](https://github.com/bolcom/unFTP/pull/130) Fixed an issue where SITE MD5 was always disabled
- [#127](https://github.com/bolcom/unFTP/pull/127) Removed unneeded [tokio](https://crates.io/crates/tokio) features
- Upgraded to Rust 1.64.0
- Upgraded dependencies

## 2022-06-26 unftp v0.13.2

- Added support for [tokio-console])(https://github.com/tokio-rs/console), the debugger for async Rust. Enable through
  the `tokio_console` compile time feature.
- [#414](https://github.com/bolcom/libunftp/pull/414) via libunftp: Fixed path display issues for Windows clients.
- [#413](https://github.com/bolcom/libunftp/pull/413) via libunftp: Fixed issue where the `OPTS UTF8` command was not
  handled correctly as seen with the FTP client included in Windows Explorer.
- Upgraded dependencies

## 2022-04-15 unftp v0.13.1

- [#343](https://github.com/bolcom/libunftp/pull/343), Added anti - brute force password guessing feature. Choose from
  different failed login attempts policies with `--failed-logins-policy [policy]`: deters successive failed login
  attempts based on `ip`, `username` or the `combination`. Default is `combination`. The maximum number of failed
  logins (`--failed-max-attempts`) and the time in seconds to unblock (`--failed-expire-after`) are also
  configurable.

## 2022-01-28 unftp v0.13.0

- BREAKING: Changed the format of the message sent in the Google Pub/Sub notifications.
- Expanded on messages sent in Google Pub/Sub notifications:
    1. Include a Logout event
    2. Added a Trace ID field to allowing matching messages pertaining to the same control channel session
    3. Added a sequence number field to allow message ordering.
    4. Added the event type as a message attribute to allow for pub/sub message filtering

  The message data format is documented in our user documentation at [unftp.rs](https://unftp.rs/server/pubsub).

## 2021-11-19 unftp v0.12.13

- Implemented integration with Google Pub/Sub through the `--ntf-pubsub-project` and `ntf-pubsub-topic` arguments.
  Configuring
  this will send notifications to the pub/sub topic for FTP file system changes and logins for instance.
- \#33 Implemented graceful shutdown
- Upgraded dependencies

## 2021-09-25 unftp v0.12.12

_tag: v0.12.12_

- Upgraded dependencies, including the latest libunftp

## 2021-07-16 unftp v0.12.11

_tag: v0.12.11_

- Added the `--usr-json-path` argument to allow per-user settings to be specified in a JSON file. This can be the same
  JSON file specified for `--auth-json-path`. See the project README for examples.
- \#85 Ability to restrict the file system operations that an FTP user can do. Accomplished with above-mentioned per
  user
  settings (`vfs_perms` property).
- \#85 Ability to specify a separate root directory per user account (`root` property).
- Ability to enable/disable an FTP account. Accomplished with above-mentioned per user settings (`account_enabled`
  property).
- \#87 Added ability to enforce mTLS per user (`client_cert` property).
- \#87 Added ability to check the CN of a user's client certificate (`client_cert.allowed_cn` property).
- Upgraded to the latest libunftp and its extentions.
  See [the libunftp changelog](https://github.com/bolcom/libunftp/blob/master/CHANGELOG.md)
  for more info.

## 2021-05-26 unftp v0.12.10

_tag: v0.12.10_

- Fixed a bug where logging to Redis stops after some time.
- Added support for the `SITE MD5` command. Use `--enable-sitemd5` for the filesystem backend (it is automatically
  enabled for the GCS storage backend)

## 2021-05-02 unftp v0.12.9

_tag: v0.12.9_

- Added Mutual TLS support with the addition of the `--ftps-client-auth` and `--ftps-trust-store` arguments.
- The JSON authentication method (`--auth-json-path`) now supports encryption through
  [PBKDF2](https://tools.ietf.org/html/rfc2898#section-5.2) encoded passwords. See the
  [unftp-auth-jsonfile](https://docs.rs/unftp-auth-jsonfile/0.1.1/unftp_auth_jsonfile/) documentation for an example.
