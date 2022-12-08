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

## Generating secure passwords

We provide a tool that generates a secure password for you.

Generating a secure password is as simple as this:

```shell
➜ docker run -ti bolcom/unftp-key-generator -u

Enter username or press ENTER to finish: hannes
Enter password or press ENTER to generate one:
Generated password: 4?KH[FN=W@bztq%[
[
  {
    "username": "hannes",
    "pbkdf2_salt": "+uhutJYSS7Y=",
    "pbkdf2_key": "sXdS1w0cH+bsNLKwW/Mek0hGXoJr+hrBJ1AjkubePiM=",
    "pbkdf2_iter": 500000
  }
]
```

Then add it to the JSON credentials file. Notice the use of the `pbkdf2_salt`, `pbkdf2_key` and `pbkdf2_iter` fields 
instead of the `password` field.

```json
[
  {
    "username": "alice",
    "password": "12345678"
  },
  {
    "username": "bob",
    "password": "secret"
  },
  {
    "username": "carol",
    "pbkdf2_salt": "Hp1WZRnzOUM=",
    "pbkdf2_key": "BOipkps/qYxlMLiuFcRjYUKivclvVXsc8f0T2pIvG6U=",
    "pbkdf2_iter": 500000
  }
]
```

For more advanced usage see the documentation of the [unftp_auth_jsonfile](https://docs.rs/unftp-auth-jsonfile/0.2.1/unftp_auth_jsonfile)
crate. The key generator tool also have advanced options that you can see by running the container with the `-h` option i.e.

```shell
docker run -ti bolcom/unftp-key-generator -h
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

## Compressing configuration files

Since unFTP v0.14.0, the `auth-json-path` and `usr-json-path` also support JSON files that are compressed with gzip, or gzip+base64-encoded.

This comes in handy when your storage is limited, and you have many users in your configuration.
When running unFTP as a container in a [Kubernetes](https://kubernetes.io/) Pod for example.
In such a setup you may have your JSON credentials file mapped into your pod via a [ConfigMap](https://kubernetes.io/docs/concepts/configuration/configmap/) or a [Secret](https://kubernetes.io/docs/concepts/configuration/secret/).
The size of these resources is limited.
By compressing the file you can grow to a larger number of users before you technically require an external database solution.
