# Changelog

## 2021-09-25 unftp v0.12.12

_tag: v0.12.12_

- Upgraded dependencies, including the latest libunftp

## 2021-07-16 unftp v0.12.11

_tag: v0.12.11_

- Added the `--usr-json-path` argument to allow per-user settings to be specified in a JSON file. This can be the same 
  JSON file specified for `--auth-json-path`. See the project README for examples.
- \#85 Ability to restrict the file system operations that an FTP user can do. Accomplished with above-mentioned per user 
  settings (`vfs_perms` property).
- \#85 Ability to specify a separate root directory per user account (`root` property). 
- Ability to enable/disable an FTP account. Accomplished with above-mentioned per user settings (`account_enabled` property).
- \#87 Added ability to enforce mTLS per user (`client_cert` property).
- \#87 Added ability to check the CN of a user's client certificate (`client_cert.allowed_cn` property).  
- Upgraded to the latest libunftp and its extentions. See [the libunftp changelog](https://github.com/bolcom/libunftp/blob/master/CHANGELOG.md) 
  for more info. 

## 2021-05-26 unftp v0.12.10

_tag: v0.12.10_

- Fixed a bug where logging to Redis stops after some time.
- Added support for the `SITE MD5` command. Use `--enable-sitemd5` for the filesystem backend (it is automatically enabled for the GCS storage backend)

## 2021-05-02 unftp v0.12.9

_tag: v0.12.9_

- Added Mutual TLS support with the addition of the `--ftps-client-auth` and `--ftps-trust-store` arguments.
- The JSON authentication method (`--auth-json-path`) now supports encryption through 
  [PBKDF2](https://tools.ietf.org/html/rfc2898#section-5.2) encoded passwords. See the 
  [unftp-auth-jsonfile](https://docs.rs/unftp-auth-jsonfile/0.1.1/unftp_auth_jsonfile/) documentation for an example.
