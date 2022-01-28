---
title: Per User Config
---

## Authentication setup

Create a credentials file (e.g. credentials.json):

```json
[
  {
    "username": "alice",
    "password": "12345678"
  },
  {
    "username": "bob",
    "password": "secret"
  }
]
```

```sh
unftp \
    --auth-type=json \
    --auth-json-path=credentials.json
```

## Per-User Settings

To do per-user settings you can expand the above-mentioned JSON file to also include some per user settings:

```json
[
  {
    "username": "alice",
    "password": "12345678",
    "vfs_perms": ["-mkdir","-rmdir","-del","-ren", "-md5"],
    "root": "alice",
    "account_enabled": true
  },
  {
    "username": "bob",
    "password": "secret",
    "client_cert": {
      "allowed_cn": "bob-the-builder"
    }
  },
  {
    "username": "vincent",
    "root": "vincent",
    "vfs_perms": ["none", "+put", "+md5"],
    "client_cert": {}
  }  
]
```

And let unFTP point to it:

```sh
unftp \
    --auth-type=json \
    --auth-json-path=users.json \
    --usr-json-path=users.json \
    ...
```

In the above configuration we use:

* `vfs_perms` - Specifies what permissions users can have. Alice cannot create directories, remove them, delete files nor 
  calculate the md5 of files. Bob can do everything while Vincent can only do uploads and calculate md5 files. Valid values
  here are "none", "all", "-mkdir, "-rmdir, "-del","-ren", "-md5", "-get", "-put", "-list", "+mkdir", "+rmdir", "+del", 
  "+ren", "+md5", "+get", "+put" and "+list".
* `root` - Sets the home directory of the user relative to the storage back-end root. Alice can only see files inside 
  `$SB_ROOT/alice`, Bob can see all files and Vincent thinks `$SB_ROOT/vincent` is the FTP root similar to Alice.
* `account_enabled` - Allows to disable the user's account completely
* `client_cert` - Allows specifying whether a client certificate is required and how to handle it. Alice logs in with 
  normal user/password authentication. No client certificate needed. Bob needs to provide a valid client certificate 
  with common name (CN) containing, 'bob-the-builder' and also needs to provide a password. Vincent can do passwordless 
  login when providing a valid certificate.
