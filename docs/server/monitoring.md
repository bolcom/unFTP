---
title: Monitoring
---

This page explains how to enable Prometheus metrics collection in unFTP by configuring the HTTP interface.

# Monitoring with Prometheus

To allow your [Prometheus](https://prometheus.io) instance to scrape metrics from unFTP you have to enable unFTP's HTTP
interface. This is done by specifying an HTTP bind address with the `--bind-address-http` argument or the 
`UNFTP_BIND_ADDRESS_HTTP` environment variable. For example if you do:

```sh
unftp \
  --bind-address=0.0.0.0:2121 \
  --bind-address-http=0.0.0.0:8080 \
  --root-dir=/home/unftp/data
```

Your will have Prometheus metrics exposed on all IP addresses and port 8080 at endpoint `http://../metrics`. 

Doing this will also expose an unFTP service information page at the HTTP root path.

Now that we've covered monitoring, you may want to configure [FTPS/TLS encryption](/server/ftps) or set up [cloud storage backends](/server/cloud-storage).
