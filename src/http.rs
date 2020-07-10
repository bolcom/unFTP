//! Contains code pertaining to unFTPs HTTP service it exposes, including prometheus metrics.
use crate::{app, metrics};

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, StatusCode,
};
use slog::*;
use std::{net::SocketAddr, result::Result};

const PATH_HOME: &str = "/";
const PATH_METRICS: &str = "/metrics";
const PATH_HEALTH: &str = "/health";
const PATH_READINESS: &str = "/ready";

// starts an HTTP server and exports Prometheus metrics.
pub async fn start(log: &Logger, bind_addr: &str) -> Result<(), String> {
    let http_addr: SocketAddr = bind_addr
        .parse()
        .map_err(|e| format!("unable to parse HTTP address {}: {}", bind_addr, e))?;

    let make_svc = make_service_fn(|_conn| {
        async {
            // service_fn converts our function into a `Service`
            Ok::<_, hyper::Error>(service_fn(router))
        }
    });

    let http_server = hyper::Server::bind(&http_addr).serve(make_svc);

    info!(log, "Starting HTTP service."; "address" => &http_addr);
    info!(log, "Exposing {} service home.", app::NAME; "path" => PATH_HOME);
    info!(log, "Exposing Prometheus {} exporter endpoint.", app::NAME; "path" => PATH_METRICS);
    info!(log, "Exposing readiness endpoint."; "path" => PATH_READINESS);
    info!(log, "Exposing liveness endpoint."; "path" => PATH_HEALTH);

    if let Err(e) = http_server.await {
        error!(log, "HTTP server error: {}", e)
    }
    Ok(())
}

fn service_home() -> Body {
    let index_html = include_str!(concat!(env!("PROJ_WEB_DIR"), "/index.html"));
    Body::from(index_html.replace("{{ .AppVersion }}", app::VERSION))
}

async fn router(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let mut response: Response<Body> = Response::new(Body::empty());
    match (req.method(), req.uri().path()) {
        (&Method::GET, PATH_HOME) | (&Method::GET, "/index.html") => {
            *response.body_mut() = service_home();
        }
        (&Method::GET, PATH_METRICS) => {
            *response.body_mut() = Body::from(metrics::gather());
        }
        (&Method::GET, PATH_READINESS) => {
            *response.body_mut() = Body::from("<html>Ready!</html>");
            *response.status_mut() = StatusCode::OK;
        }
        (&Method::GET, PATH_HEALTH) => {
            *response.body_mut() = Body::from("<html>OK!</html>");
            *response.status_mut() = StatusCode::OK;
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    }

    Ok(response)
}
