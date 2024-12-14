---
title: unFTP Up
---

Arguments can be given to unFTP either via the command line or via environment variables. To show a list of available
program arguments type the following:

```sh
unftp --help
```

To run passwordless unFTP with everything set to the defaults:

```sh
unftp --auth-type anonymous
```

{% info Note %}
The `--auth-type` switch is required, but will be omitted in many other examples for brevity.
Choose the appropriate authentication type for your use case—do not use anonymous unless explicitly intended.
{% end %}

This will start unFTP:

- with a filesystem back-end rooted at your OS temporary directory.
- the FTP control channel bound to `0.0.0.0:2121` and the data channel range set to `0.0.0.0` ports `49152`..`65535`
- authentication being anonymous i.e. clients can specify any username and password

Now serving data from your computer's temporary directory is not very useful so lets point it to a different directory
by using the `--root-dir` argument. In addition, lets limit the data port range a bit:

```sh
unftp \
  -v \
  --root-dir=/home/unftp/data \
  --bind-address=0.0.0.0:2121 \
  --passive-ports=50000-51000
```
