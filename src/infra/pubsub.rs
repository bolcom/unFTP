use crate::domain::events::{EventDispatcher, FTPEvent, FTPEventPayload};
use crate::infra::workload_identity;
use async_trait::async_trait;
use base64::Engine;
use http::{header, Method, Request, StatusCode, Uri};
use http_body_util::{Either, Empty};
use hyper::body::Bytes;
use hyper::body::Incoming;
use hyper::Response;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// Notes:
//  - Emulator: https://cloud.google.com/pubsub/docs/emulator
//  - virtualenv -p python3 mypython
//  - API Docs for publishing: https://cloud.google.com/pubsub/docs/reference/rest/v1/projects.topics/publish
//

/// An [EventDispatcher] that dispatches to Google Pub/sub
#[derive(Debug)]
pub struct PubsubEventDispatcher {
    log: Arc<slog::Logger>,
    api_base_url: String,
    project: String,
    topic: String,
    client: Client<HttpsConnector<HttpConnector>, Either<String, Empty<Bytes>>>,
}

const DEFAULT_SERVICE_ENDPOINT: &str = "https://pubsub.googleapis.com";

impl PubsubEventDispatcher {
    #[allow(dead_code)]
    pub fn new<Str>(log: Arc<slog::Logger>, project: Str, topic: Str) -> Self
    where
        Str: Into<String>,
    {
        Self::with_api_base(
            log,
            project.into(),
            topic.into(),
            DEFAULT_SERVICE_ENDPOINT.to_owned(),
        )
    }

    pub fn with_api_base<Str>(
        log: Arc<slog::Logger>,
        project: Str,
        topic: Str,
        api_base: Str,
    ) -> Self
    where
        Str: Into<String>,
    {
        let https = HttpsConnectorBuilder::new()
            .with_native_roots()
            .expect("no native root CA certificates found")
            .https_or_http()
            .enable_http1()
            .build();

        let client = Client::builder(TokioExecutor::new()).build(https);

        PubsubEventDispatcher {
            log,
            api_base_url: api_base.into(),
            project: project.into(),
            topic: topic.into(),
            client,
        }
    }

    // Gets the authentication token through workload identity mechanisms
    async fn get_token(&self) -> Result<String, workload_identity::Error> {
        Ok(workload_identity::request_token(None, self.client.clone())
            .await?
            .access_token)
    }

    fn event_type(event: FTPEventPayload) -> String {
        String::from(match event {
            FTPEventPayload::Startup { .. } => "startup",
            FTPEventPayload::Login { .. } => "login",
            FTPEventPayload::Logout { .. } => "logout",
            FTPEventPayload::Get { .. } => "get",
            FTPEventPayload::Put { .. } => "put",
            FTPEventPayload::Delete { .. } => "delete",
            FTPEventPayload::MakeDir { .. } => "makeDir",
            FTPEventPayload::Rename { .. } => "rename",
            FTPEventPayload::RemoveDir { .. } => "removeDir",
        })
    }

    // publishes to Google pub/sub
    async fn publish(&self, event: FTPEvent) -> Result<(), String> {
        let msg = base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_string(&event).unwrap());
        let b = PubSubRequest {
            messages: vec![PubSubMsg {
                data: msg.to_owned(),
                attributes: HashMap::from([(
                    String::from("eventType"),
                    Self::event_type(event.payload),
                )]),
            }],
        };
        let body_string =
            serde_json::to_string(&b).map_err(|e| format!("error marshalling message: {}", e))?;

        // TODO: Implement other auth methods
        // FIXME: When testing locally there won't be a token, we might want to handle this better.
        let token = self.get_token().await.unwrap_or_else(|_| "".to_owned());

        let request: Request<Either<String, Empty<Bytes>>> = Request::builder()
            .uri(
                Uri::from_maybe_shared(format!(
                    "{}/v1/projects/{}/topics/{}:publish",
                    self.api_base_url, self.project, self.topic
                ))
                .map_err(|e| format!("invalid request URI: {}", e))?,
            )
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .method(Method::POST)
            .body(Either::Left(body_string))
            .map_err(|e| format!("error with publish request: {}", e))?;

        let response: Response<Incoming> = self.client.request(request).await.unwrap();
        if response.status() != StatusCode::OK {
            Err(format!(
                "bad HTTP status code received: {}",
                response.status()
            ))
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl EventDispatcher<FTPEvent> for PubsubEventDispatcher {
    async fn dispatch(&self, event: FTPEvent) {
        let r = self.publish(event).await;
        if r.is_err() {
            slog::error!(
                self.log,
                "Could not dispatch event to pub/sub: {}",
                r.unwrap_err()
            );
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PubSubRequest {
    messages: Vec<PubSubMsg>,
}

#[derive(Serialize, Deserialize)]
struct PubSubMsg {
    data: String,
    attributes: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use base64::engine::general_purpose;
    use base64::Engine as _;

    use crate::infra::pubsub::{PubSubMsg, PubSubRequest};
    use base64::Engine;
    use std::collections::HashMap;

    #[test]
    fn pubub_request_serializes_correctly() {
        let payload = general_purpose::STANDARD.encode(b"123");
        let r = PubSubRequest {
            messages: vec![PubSubMsg {
                data: payload,
                attributes: HashMap::new(),
            }],
        };
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(
            json,
            "{\"messages\":[{\"data\":\"MTIz\",\"attributes\":{}}]}"
        )
    }
}
