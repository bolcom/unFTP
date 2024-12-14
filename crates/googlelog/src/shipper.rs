use std::sync::mpsc as sync_mpsc;
use tokio::sync::mpsc as async_mpsc;

use google_logging2::api::WriteLogEntriesRequest;

use chrono::{DateTime, TimeDelta, Utc};

use reqwest::{Client, Response};

use crate::error::Error;

/// Token caching
#[derive(Default)]
pub struct Token {
    token: Option<String>,
    renew_after: DateTime<Utc>,
}

async fn get_error_response(response: Response, context: String) -> Error {
    let status = response.status();

    let body = match response.bytes().await {
        Ok(bytes) => match serde_json::from_slice::<String>(&bytes) {
            Ok(json) => json,
            Err(_) => String::from_utf8_lossy(&bytes).to_string(),
        },
        Err(e) => format!("could not decode body of HTTP Error response: {e}"),
    };

    Error::HttpResponseError {
        context,
        status,
        body,
    }
}

impl Token {
    fn renew_after_from_expires_in(expires_in: u64) -> DateTime<Utc> {
        let renew_after = TimeDelta::seconds((expires_in - 60) as i64);
        Utc::now() + renew_after
    }

    async fn fetch_access_token(&mut self, client: &Client) -> Result<String, Error> {
        if let Some(token) = &self.token {
            if Utc::now() < self.renew_after {
                return Ok(token.clone());
            }
        }

        let response = client.get("http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token")
            .header("Metadata-Flavor", "Google")
            .send()
            .await
            .map_err(|e| {
            Error::ShipperReqwestError {
                context: "performing HTTP GET token credentials from metadata server".to_string(),
                source: e,
            }
        })?;

        if response.status().is_success() {
            let body = response
                .text()
                .await
                .map_err(|e| Error::ShipperReqwestError {
                    context: "consuming response body of access token request".to_string(),
                    source: e,
                })?;
            let token_data: serde_json::Value =
                serde_json::from_str(&body).map_err(|e| Error::ShipperSerializeError {
                    context: "deserializing token data".to_string(),
                    source: e,
                })?;
            let token_str = token_data["access_token"].as_str().unwrap().to_string();
            self.token = Some(token_str.clone());
            self.renew_after =
                Self::renew_after_from_expires_in(token_data["expires_in"].as_u64().unwrap());
            Ok(token_str)
        } else {
            Err(get_error_response(response, "fetching token".to_string()).await)
        }
    }
}

/// A sync to async channel bridge.
/// Forwards the log messages from the [Drain's log function](crate::logger::Logger) to the [`Shipper`]
pub struct Bridge {
    sync_rx: sync_mpsc::Receiver<WriteLogEntriesRequest>,
    async_tx: async_mpsc::Sender<WriteLogEntriesRequest>,
}

impl Bridge {
    /// Forwards log messages from the drain to the shipper
    /// For usage see the [`example`](crate::logger::Builder::build_with_async_shipper).
    pub fn run_sync_to_async_bridge(self) {
        while let Ok(message) = self.sync_rx.recv() {
            let tx = self.async_tx.clone();
            tokio::task::spawn(async move {
                if let Err(e) = tx.send(message).await {
                    eprintln!("Failed to forward log message to async channel, log message not sent to Google Logger: {}", e);
                }
            });
        }
    }
}

/// Sends the log messages to the Google Logging API
pub struct Shipper {
    client: Client,
    token: Token,
    sync_rx: Option<sync_mpsc::Receiver<WriteLogEntriesRequest>>,
    async_rx: async_mpsc::Receiver<WriteLogEntriesRequest>,
    async_tx: Option<async_mpsc::Sender<WriteLogEntriesRequest>>,
}

impl Shipper {
    /// Takes the sync receiver and async sender from the Shipper struct into the [`Bridge`]
    /// For usage see the [`example`](crate::logger::Builder::build_with_async_shipper).
    pub fn yield_bridge(&mut self) -> Bridge {
        match (self.sync_rx.take(), self.async_tx.take()) {
            (Some(sync_rx), Some(async_tx)) => Bridge { sync_rx, async_tx },
            (_, _) => panic!("May not happen"),
        }
    }

    /// Creates a `Shipper`
    pub fn new(sync_rx: sync_mpsc::Receiver<WriteLogEntriesRequest>) -> Self {
        let (async_tx, async_rx) = tokio::sync::mpsc::channel::<WriteLogEntriesRequest>(100);

        Shipper {
            client: Client::new(),
            token: Token::default(),
            sync_rx: Some(sync_rx),
            async_rx,
            async_tx: Some(async_tx),
        }
    }

    async fn send_log_entry(
        &mut self,
        token: &str,
        body: WriteLogEntriesRequest,
    ) -> Result<(), Error> {
        let url = "https://logging.googleapis.com/v2/entries:write".to_string();

        let response = self
            .client
            .post(url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::ShipperReqwestError {
                context: "performing HTTP POST request to the Google Logging API".to_string(),
                source: e,
            })?;
        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(get_error_response(
                response,
                "response when sending log entry to Google Logging API".to_string(),
            )
            .await)
        }
    }

    /// The process that receives log entries and sends them to the Google Logging API
    pub async fn run_log_shipper(mut self) {
        while let Some(log_entry) = self.async_rx.recv().await {
            match self.token.fetch_access_token(&self.client).await {
                Ok(token) => {
                    if let Err(e) = self.send_log_entry(&token, log_entry).await {
                        eprintln!("Failed to send log entry: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to fetch access token: {}", e);
                }
            }
        }
    }
}
