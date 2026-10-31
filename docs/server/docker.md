---
title: Docker
---

This page explains how to run unFTP using Docker, including downloading pre-built images and configuring the container with environment variables and volume mounts.

Starting with the next release, pre-made images will be published on
[Docker Hub](https://hub.docker.com/r/cloudinvoke/unftp/tags) as `cloudinvoke/unftp` and on
GitHub Container Registry as `ghcr.io/cloudinvoke/unftp`. For example, with a released version:

```sh
docker pull cloudinvoke/unftp:vX.Y.Z-alpine
docker pull ghcr.io/cloudinvoke/unftp:vX.Y.Z-scratch
```

Older images remain available from the former
[Docker Hub repository](https://hub.docker.com/r/bolcom/unftp/tags).

Releases with ARM64 support publish the `-alpine`, `-scratch`, and `-alpine-debug` image tags for both `linux/amd64`
and `linux/arm64`. Docker selects the matching image for your host when you pull a tag. Older tags may support only
`linux/amd64`.

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
  cloudinvoke/unftp:vX.Y.Z-alpine
```

Now that we've covered Docker deployment, you may want to configure [cloud storage backends](/server/cloud-storage) or set up [monitoring](/server/monitoring) for your containerized deployment.
