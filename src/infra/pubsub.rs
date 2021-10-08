use crate::domain::{EventDispatcher, FTPEvent};
use crate::infra::workload_identity;
use async_trait::async_trait;
use http::{header, Method, Request, StatusCode, Uri};
use hyper::client::connect::dns::GaiResolver;
use hyper::client::HttpConnector;
use hyper::{Body, Client, Response};
use hyper_rustls::HttpsConnector;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// Notes:
//  - Emulator: https://cloud.google.com/pubsub/docs/emulator
//  - virtualenv -p python3 mypython
//  - API Docs for publishing: https://cloud.google.com/pubsub/docs/reference/rest/v1/projects.topics/publish
//

/// An [EventDispatcher](crate::domain::EventDispatcher) that dispatches to Google Pub/sub
#[derive(Debug)]
pub struct PubsubEventDispatcher {
    log: Arc<slog::Logger>,
    api_base_url: String,
    project: String,
    topic: String,
    client: Client<HttpsConnector<HttpConnector>>,
}

const DEFAULT_SERVICE_ENDPOINT: &str = "https://pubsub.googleapis.com";

impl PubsubEventDispatcher {
    #[allow(dead_code)]
    pub fn new<Str>(log: Arc<slog::Logger>, project: Str, topic: Str) -> Self
    where
        Str: Into<String>,
    {
        Self::with_api_base(log, project.into(), topic.into(), DEFAULT_SERVICE_ENDPOINT.to_owned())
    }

    pub fn with_api_base<Str>(log: Arc<slog::Logger>, project: Str, topic: Str, api_base: Str) -> Self
    where
        Str: Into<String>,
    {
        let client: Client<HttpsConnector<HttpConnector<GaiResolver>>, Body> =
            Client::builder().build(HttpsConnector::with_native_roots());
        PubsubEventDispatcher {
            log,
            api_base_url: api_base.into(),
            project: project.into(),
            topic: topic.into(),
            client,
        }
    }
}

impl PubsubEventDispatcher {
    // Gets the authentication token through workload identity mechanisms
    async fn get_token(&self) -> Result<String, workload_identity::Error> {
        Ok(workload_identity::request_token(None, self.client.clone())
            .await?
            .access_token)
    }

    // publishes to Google pub/sub
    async fn publish(&self, event: FTPEvent) -> Result<(), String> {
        let msg = base64::encode(serde_json::to_string(&event).unwrap());
        let b = PubSubRequest {
            messages: vec![PubSubMsg {
                data: msg.to_owned(),
                attributes: HashMap::new(), // TODO Set attribute based on the event type so subscribers can filter.
            }],
        };
        let body_string = serde_json::to_string(&b).map_err(|e| format!("error marshalling message: {}", e))?;

        // TODO: Implement other auth methods
        // FIXME: When testing locally there won't be a token, we might want to handle this better.
        let token = self.get_token().await.unwrap_or_else(|_| "".to_owned());

        let request: Request<Body> = Request::builder()
            .uri(
                Uri::from_maybe_shared(format!(
                    "{}/v1/projects/{}/topics/{}:publish",
                    self.api_base_url, self.project, self.topic
                ))
                .map_err(|e| format!("invalid request URI: {}", e))?,
            )
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .method(Method::POST)
            .body(body_string.into())
            .map_err(|e| format!("error with publish request: {}", e))?;

        let response: Response<Body> = self.client.request(request).await.unwrap();
        if response.status() != StatusCode::OK {
            Err(format!("bad HTTP status code received: {}", response.status()))
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
            slog::error!(self.log, "Could not dispatch event to pub/sub: {}", r.unwrap_err());
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
    use crate::infra::pubsub::{PubSubMsg, PubSubRequest};
    use std::collections::HashMap;

    #[test]
    fn pubub_request_serializes_correctly() {
        let payload = base64::encode("123");
        let r = PubSubRequest {
            messages: vec![PubSubMsg {
                data: payload.to_owned(),
                attributes: HashMap::new(),
            }],
        };
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, "{\"messages\":[{\"data\":\"MTIz\",\"attributes\":{}}]}")
    }
}
