---
title: Cloud Storage
---

This page explains how to configure unFTP to use cloud storage backends, including Google Cloud Storage (GCS) and Azure Blob Storage, instead of traditional filesystem storage.

You can run unFTP with a traditional file system back-end on a bare metal or virtual machine but if you're building your 
solutions in the cloud it would make sense to store your files in blob storage. unFTP comes with integration to [Google 
Cloud Storage (GCS)](https://cloud.google.com/storage) and [Azure Blob Storage](https://azure.microsoft.com/en-us/products/storage/blobs).

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

# Using Azure Blob Storage

You can enable the Azure Blob Storage backend by setting the storage back-end type (`--sbe-type`) to `azblob`. 

Azure Blob Storage support is provided via [Apache OpenDAL](https://opendal.apache.org/) through the [`unftp-sbe-opendal`](https://crates.io/crates/unftp-sbe-opendal) storage back-end crate. 

## Required Configuration

The following arguments are required to use Azure Blob Storage:

- `--sbe-opendal-azblob-container`: The name of the Azure Blob Storage container
- `--sbe-opendal-azblob-account-name`: The Azure Storage account name

## Authentication

You can authenticate using either:

- **Account Key**: Set `--sbe-opendal-azblob-account-key` with your Azure Storage account key
- **SAS Token**: Set `--sbe-opendal-azblob-sas-token` with a [Shared Access Signature (SAS) token](https://learn.microsoft.com/en-us/azure/storage/common/storage-sas-overview)

If neither is provided via command line arguments, unFTP will attempt to load credentials from environment variables.

## Optional Configuration

- `--sbe-opendal-azblob-root`: Root path within the container. All operations will happen under this root. (default: root of container)
- `--sbe-opendal-azblob-endpoint`: Custom endpoint URL. Must be a full URI. (default: standard Azure Blob Storage endpoint)
- `--sbe-opendal-azblob-batch-max-operations`: Maximum number of operations in a batch request

## Example

Using account key authentication:

```sh
unftp \
  --sbe-type=azblob \
  --sbe-opendal-azblob-container=mycontainer \
  --sbe-opendal-azblob-account-name=mystorageaccount \
  --sbe-opendal-azblob-account-key=your-account-key \
  --sbe-opendal-azblob-root=ftp-data
```

Using SAS token authentication:

```sh
unftp \
  --sbe-type=azblob \
  --sbe-opendal-azblob-container=mycontainer \
  --sbe-opendal-azblob-account-name=mystorageaccount \
  --sbe-opendal-azblob-sas-token="?sv=2021-06-08&ss=bfqt&srt=sco&sp=rwdlacupx&se=..."
```

Now that we've covered cloud storage backends, you may want to configure [authentication](/server/jsonconfig) or set up [logging](/server/logging) for your cloud deployment.

