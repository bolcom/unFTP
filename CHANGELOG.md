# Changelog

## Upcoming

- SITE MD5 support added, use `--enable-sitemd5` for the filesystem backend (it is automatically enabled for the GCS storage backend)

## 2021-05-02 unftp v0.12.9

_tag: v0.12.9_

- Added Mutual TLS support with the addition of the `--ftps-client-auth` and `--ftps-trust-store` arguments.
- The JSON authentication method (`--auth-json-path`) now supports encryption through 
  [PBKDF2](https://tools.ietf.org/html/rfc2898#section-5.2) encoded passwords. See the 
  [unftp-auth-jsonfile](https://docs.rs/unftp-auth-jsonfile/0.1.1/unftp_auth_jsonfile/) documentation for an example.
