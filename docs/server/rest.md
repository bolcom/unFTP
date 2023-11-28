---
title: REST auth
---

There are multiple ways to externalize authentication, rather than [local JSON authentication](/server/jsonconfig).
This page explains how to use the REST authenticator.
With the REST method, you provide your own HTTP endpoint.
You can use any method like GET or POST, and you can customize unFTP to support your API.

## Set up

Example:

```sh
unftp \
    --auth-type rest \
    --auth-rest-method POST \
    --auth-rest-url http://localhost:5000/v1/ftp-auth \
    --auth-rest-body '{"username":"{USER}","password":"{PASS}"}' \
    --auth-rest-selector /status \
    --auth-rest-regex successful
```

Let's say, a user `alice` logs in on the unFTP server.
With the above configuration unFTP will build an HTTP POST authentication request for http://localhost:5000/v1/ftp-auth
It will replace the placeholders `{USER}`, and `{PASS}` with the given FTP username and password:

```json
{"username":"alice","password":"abc1234"}
```

And, if the login was successful, the server should respond with something like:

```json
{"message":"User logged in.","status":"successful"}
```

The REST authenticator uses the `/status` JSON pointer, and matches it to `"successful"`.

Aside from the placeholders `{USER}` and `{PASS}` you can use `{IP}`.
That will add the source IP address of the connected client.
That is in case if you want to perform client-IP whitelisting, next to regular username and password.

