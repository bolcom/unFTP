# slog-google

A [slog](https://crates.io/crates/slog) drain that writes [Google Cloud Logging](https://cloud.google.com/logging) entries.

There are two ways to ship the entries:

1. `build()` — receive `WriteLogEntries` on a channel and handle transport yourself.
2. `build_with_async_shipper()` — send them to the Logging API from an async task. Enable the `shipper` feature for this.

```toml
[dependencies]
slog-google = { version = "0.1.0", features = ["shipper"] }
```

```rust
use serde_json::json;
use slog::*;
use slog_google::logger::Builder;

let resource_labels = json!({
    "project_id": "my-gcp-project",
    "location": "europe-west1-b",
    "cluster_name": "my-gke-cluster",
    "namespace_name": "my-gke-namespace",
    "pod_name": std::env::var("HOSTNAME").unwrap_or_default(),
    "container_name": "my-app",
});

let (drain, mut shipper) = Builder::new(
    "projects/my-gcp-project/logs/my-log-id",
    "k8s_container",
)
.with_resource_labels(resource_labels)
.unwrap()
.build_with_async_shipper();

let bridge = shipper.yield_bridge();
tokio::task::spawn_blocking(move || {
    bridge.run_sync_to_async_bridge();
});
tokio::spawn(async move {
    shipper.run_log_shipper().await;
});

let log = Logger::root(drain.fuse(), o!());
info!(log, "hello from GCP"; "msg" => "Hello World!");
```

The shipper fetches a token from the [GCE metadata server](https://cloud.google.com/compute/docs/metadata/overview) as the default service account. That works on GCE, GKE, Cloud Run, and similar GCP environments. `build()` does not authenticate; you ship the entries yourself.
