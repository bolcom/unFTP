---
title: Docker
---

You can download pre-made docker images from [docker hub](https://hub.docker.com/r/bolcom/unftp/tags) e.g.:

```sh
docker pull bolcom/unftp:v0.13.4-alpine
docker pull bolcom/unftp:v0.13.4-alpine-istio
docker pull bolcom/unftp:v0.13.4-scratch
```

Example running unFTP in a Docker container:

```sh
docker run \
  -e ROOT_DIR=/ \
  -e UNFTP_LOG_LEVEL=info \
  -e UNFTP_FTPS_CERTS_FILE='/unftp.crt' \
  -e UNFTP_FTPS_KEY_FILE='/unftp.key' \
  -e UNFTP_PASSIVE_PORTS=50000-50005 \
  -e UNFTP_SBE_TYPE=gcs \
  -e UNFTP_SBE_GCS_BUCKET=the-bucket-name \
  -e UNFTP_SBE_GCS_KEY_FILE=/key.json \
  -p 2121:2121 \
  -p 50000:50000 \
  -p 50001:50001 \
  -p 50002:50002 \
  -p 50003:50003 \
  -p 50004:50004 \
  -p 50005:50005 \
  -p 8080:8080 \
  -v /Users/xxx/unftp/unftp.key:/unftp.key  \
  -v /Users/xxx/unftp/unftp.crt:/unftp.crt \
  -v /Users/xxx/unftp/the-key.json:/key.json \
  -ti \
  bolcom/unftp:v0.13.4-alpine
```

