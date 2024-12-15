#[cfg(feature = "shipper")]
use crate::shipper;

use crate::error::Error;

use google_logging2::api::{LogEntry, MonitoredResource, WriteLogEntriesRequest};

use slog::{self, Drain, Key, Level, Never, OwnedKVList, Record, KV};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Write;

use serde_json::json;

use std::sync::mpsc::sync_channel;

use chrono::Utc;

/// Builder for the [`Logger`]
#[derive(Default, Debug)]
pub struct Builder {
    log_name: String,
    log_level_label: Option<String>,
    resource_type: String,
    default_labels: HashMap<String, String>,
    resource_labels: Option<HashMap<String, String>>,
}

/// Main struct for the Google Logger drain
pub struct Logger {
    log_name: String,
    log_level_label: Option<String>,
    default_labels: HashMap<String, String>,
    resource: MonitoredResource,
    sync_tx: std::sync::mpsc::SyncSender<WriteLogEntriesRequest>,
}

impl Builder {
    /// Creates a Builder object.
    ///
    /// # Parameters
    /// - `log_name`: The `logName` string to be used in the [LogEntry](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry)
    /// - `resource_type`: The required `type` field set in the `resource` [MonitoredResource](https://cloud.google.com/logging/docs/reference/v2/rest/v2/MonitoredResource) object of the [LogEntry](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry). For example: `k8s_container`.
    ///
    /// # Example
    ///
    /// ```
    /// use slog_google::logger::Builder;
    /// let (drain, _) = Builder::new(
    ///     "projects/my-gcp-project/logs/my-log-id",
    ///     "k8s_container",
    /// )
    /// .build();
    /// ```
    ///
    #[must_use = "The builder must be used"]
    pub fn new(log_name: &str, resource_type: &str) -> Self {
        Self {
            log_name: log_name.to_string(),
            resource_type: resource_type.to_string(),
            ..Default::default()
        }
    }

    /// Sets resource labels to be applied.
    ///
    /// These labels will populate the `labels` field in the `resource` [MonitoredResource](https://cloud.google.com/logging/docs/reference/v2/rest/v2/MonitoredResource) object of the [LogEntry](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry).
    ///
    /// # Example
    ///
    /// ```
    /// use serde_json::json;
    /// let resource_labels = json!(
    /// {
    ///     "pod_name": "dummy-value",
    ///     "location": "europe-west1-b",
    ///     "pod_name": std::env::var("HOSTNAME").unwrap_or_default(),
    ///     "container_name": "my-app",
    ///     "project_id": "my-gcp-project",
    ///     "cluster_name": "my-gke-cluster",
    ///     "namespace_name": "my-gke-namespace"
    /// });
    ///
    /// use slog_google::logger::Builder;
    /// let (drain, _) = Builder::new(
    ///     "projects/my-gcp-project/logs/my-log-id",
    ///     "k8s_container",
    /// )
    /// .with_resource_labels(resource_labels)
    /// .unwrap()
    /// .build();
    /// ```
    ///
    /// # Errors
    ///
    /// Will return `Err` if `labels` does not parse as JSON.
    #[must_use = "The builder must be used"]
    pub fn with_resource_labels(
        self,
        labels: serde_json::Value,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            resource_labels: Some(
                serde_json::from_value(labels).map_err(Error::ResourceLabelsError)?,
            ),
            ..self
        })
    }

    /// Sets default labels to be applied in the labels field.
    ///
    /// These will populate the `labels` top level field of the [LogEntry](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry). These labels are added in addition to any labels set in the logger statement.
    ///
    /// # Example
    ///
    /// ```
    /// use serde_json::json;
    /// let default_labels = json!(
    /// {
    ///     "application": "my-application",
    ///     "team": "my-team",
    ///     "version": "my-app-version",
    ///     "role": "my-app-role",
    ///     "environment": "production",
    ///     "platform": "gcp",
    /// });
    /// ```
    ///
    /// # Errors
    ///
    /// Will return `Err` if `labels` does not parse as JSON.
    #[must_use = "The builder must be used"]
    pub fn with_default_labels(self, labels: serde_json::Value) -> Result<Self, Error> {
        Ok(Self {
            default_labels: serde_json::from_value(labels).map_err(Error::DefaultLabelsError)?,
            ..self
        })
    }

    /// Sets the label name to store the log level
    ///
    /// If set, the log level value is added under this label the `labels` top level field of the [LogEntry](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry)
    ///
    /// If not set, the log level is not propagated, but you will still have the [severity](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#LogSeverity), which is always there.
    #[must_use = "The builder must be used"]
    pub fn with_log_level_label(self, log_level_label: &str) -> Self {
        Self {
            log_level_label: Some(log_level_label.into()),
            ..self
        }
    }

    /// This returns a tuple with a [`Logger`](struct@Logger), which can be passed to the slog root logger [as usual](https://docs.rs/slog/latest/slog/#where-to-start), and a [`std::sync::mpsc::Receiver`] channel.
    /// The `Logger` sends the [`WriteLogEntries`](https://cloud.google.com/logging/docs/reference/v2/rpc/google.logging.v2#google.logging.v2.LoggingServiceV2.WriteLogEntries) it creates to this channel.
    ///
    /// For instance you could output these to the console, if you have an external agent that reads the process' output and ships it to Google Logging.
    ///
    #[must_use = "The logger and receiver must be used to handle logging correctly"]
    #[allow(dead_code)]
    pub fn build(self) -> (Logger, std::sync::mpsc::Receiver<WriteLogEntriesRequest>) {
        let (sync_tx, sync_rx) = sync_channel::<WriteLogEntriesRequest>(100);
        (
            Logger {
                log_name: self.log_name,
                log_level_label: self.log_level_label,
                default_labels: self.default_labels,
                resource: MonitoredResource {
                    type_: Some(self.resource_type),
                    labels: self.resource_labels,
                },
                sync_tx,
            },
            sync_rx,
        )
    }

    /// In an async context this 'shipper' sends the log entries directly to the [Google Logging API](https://cloud.google.com/logging/docs/reference/v2/rest).
    ///
    /// # Example
    ///
    /// ```
    /// use tokio::runtime::Runtime;
    /// use serde_json::json;
    ///
    /// let mut rt = Runtime::new().unwrap();
    /// rt.spawn(async {
    ///   let resource_labels = json!(
    ///   {
    ///       "pod_name": "dummy-value",
    ///       "location": "europe-west1-b",
    ///       "pod_name": std::env::var("HOSTNAME").unwrap_or_default(),
    ///       "container_name": "my-app",
    ///       "project_id": "my-gcp-project",
    ///       "cluster_name": "my-gke-cluster",
    ///       "namespace_name": "my-gke-namespace"
    ///   });
    ///
    ///   use slog_google::logger::Builder;
    ///   let (drain, mut shipper) = Builder::new(
    ///       "projects/my-gcp-project/logs/my-log-id",
    ///       "k8s_container",
    ///   )
    ///   .with_resource_labels(resource_labels)
    ///   .unwrap()
    ///   .build_with_async_shipper();
    ///
    ///   // Forward messages from the sync channel to the async channel where the
    ///   // shipper sends it to Google Cloud Logging
    ///   let bridge = shipper.yield_bridge();
    ///   tokio::task::spawn_blocking(move || {
    ///       bridge.run_sync_to_async_bridge();
    ///   });
    ///
    ///   tokio::spawn(async move {
    ///       shipper.run_log_shipper().await;
    ///   });
    /// });
    ///
    /// ```
    #[cfg(feature = "shipper")]
    #[must_use = "The logger and shipper must be used to handle logging correctly"]
    pub fn build_with_async_shipper(self) -> (Logger, shipper::Shipper) {
        let (sync_tx, sync_rx) = sync_channel::<WriteLogEntriesRequest>(100);
        (
            Logger {
                log_name: self.log_name,
                log_level_label: self.log_level_label,
                default_labels: self.default_labels,
                resource: MonitoredResource {
                    type_: Some(self.resource_type),
                    labels: self.resource_labels,
                },
                sync_tx,
            },
            shipper::Shipper::new(sync_rx),
        )
    }
}

impl Logger {
    // Determine a sensible severity based on the log level
    fn get_severity(log_level: Level) -> String {
        // https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#logseverity
        match log_level {
            Level::Critical => "CRITICAL".into(),
            Level::Error => "ERROR".into(),
            Level::Warning => "WARNING".into(),
            Level::Info => "INFO".into(),
            Level::Debug | Level::Trace => "DEBUG".into(),
        }
    }

    fn construct_log_entry(
        &self,
        message: &str,
        log_level: Level,
        serializer: Serializer,
    ) -> LogEntry {
        let mut labels = self.default_labels.clone();

        if !serializer.map.is_empty() {
            labels.extend(serializer.map);
        }

        // We add the log level to the labels if requested
        if let Some(label) = &self.log_level_label {
            labels.insert(label.clone(), log_level.as_str().to_string());
        }

        let resource = Some(self.resource.clone());

        // TODO: support both text_payload and json_payload
        let json_payload = HashMap::from([("message".to_string(), json!(message))]);

        // https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#logseverity
        LogEntry {
            json_payload: Some(json_payload),
            labels: Some(labels),
            severity: Some(Self::get_severity(log_level)),
            timestamp: Some(Utc::now()),
            resource,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
struct Serializer {
    map: HashMap<String, String>,
}

impl Serializer {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl slog::Serializer for Serializer {
    fn emit_arguments(&mut self, key: Key, val: &fmt::Arguments) -> slog::Result {
        let mut value = String::new();
        write!(value, "{val}")?;
        self.map.insert(key.into(), value);
        Ok(())
    }
}

impl Drain for Logger {
    type Ok = ();
    type Err = Never; // TODO: Handle errors

    fn log(&self, record: &Record<'_>, values: &OwnedKVList) -> Result<Self::Ok, Self::Err> {
        let mut serializer = Serializer::new();

        let kv = record.kv();
        let _ = kv.serialize(record, &mut serializer);

        let _ = values.serialize(record, &mut serializer);

        let log_entry = self.construct_log_entry(
            format!("{}", record.msg()).as_str(),
            record.level(),
            serializer,
        );

        let body = WriteLogEntriesRequest {
            log_name: Some(self.log_name.clone()),
            entries: Some(vec![log_entry]),
            ..Default::default()
        };

        let _ = self.sync_tx.send(body);

        Ok(())
    }
}
