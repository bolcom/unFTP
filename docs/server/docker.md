---
title: Docker
---

This page explains how to run unFTP using Docker, including downloading pre-built images and configuring the container with environment variables and volume mounts.

You can download pre-made docker images from [docker hub](https://hub.docker.com/r/bolcom/unftp/tags) e.g.:

```sh
docker pull bolcom/unftp:v0.15.2-alpine
docker pull bolcom/unftp:v0.15.2-scratch
```

Example running unFTP in a Docker container:

```sh
docker run \
  -e UNFTP_ROOT_DIR=/ \
  -e UNFTP_LOG_LEVEL=info \
  -e UNFTP_FTPS_CERTS_FILE='/unftp.crt' \
  -e UNFTP_FTPS_KEY_FILE='/unftp.key' \
  -e UNFTP_PASSIVE_PORTS=50000-50005 \
  -e UNFTP_SBE_TYPE=gcs \
  -e UNFTP_SBE_GCS_BUCKET=the-bucket-name \
  -e UNFTP_SBE_GCS_KEY_FILE=/key.json \
  -p 2121:2121 \
  -p 50000-50020:50000-50020 \
  -p 8080:8080 \
  -v /Users/xxx/unftp/unftp.key:/unftp.key  \
  -v /Users/xxx/unftp/unftp.crt:/unftp.crt \
  -v /Users/xxx/unftp/the-key.json:/key.json \
  -ti \
  bolcom/unftp:v0.15.2-alpine
```

Now that we've covered Docker deployment, you may want to configure [cloud storage backends](/server/cloud-storage) or set up [monitoring](/server/monitoring) for your containerized deployment.
