---
title: Cloud Storage
---

You can run unFTP with a traditional file system back-end on a bare metal or virtual machine but if you're building your 
solutions in the cloud it would make sense to store your files in blob storage. unFTP comes with integration to [Google 
Cloud Storage (GCS)](https://cloud.google.com/storage).

# Using the GCS Back-end

You can enable the GCS backend by setting the storage back-end type (`--sbe-type`) to `gcs`. For storage authentication 
you can choose between [workload identity](https://cloud.google.com/kubernetes-engine/docs/how-to/workload-identity) and 
using a service account key file. For the former set the `--sbe-gcs-service-account` argument and for the latter 
the `--sbe-gcs-key-file` argument.

The storage bucket name is chosen through `--sbe-gcs-bucket` and the root path inside the bucket through 
`--sbe-gcs-root`. 

Here is an example:

```sh
unftp \
  --sbe-type=gcs \
  --sbe-gcs-bucket=mybucket \
  --sbe-gcs-root=ftp-base \
  --sbe-gcs-key-file=/path/to/file
```
