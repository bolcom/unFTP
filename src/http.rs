//! Contains code pertaining to unFTPs HTTP service it exposes, including prometheus metrics.
use crate::{app, metrics};

use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, StatusCode,
};
use slog::*;
use std::net::{IpAddr, Ipv4Addr};
use std::{net::SocketAddr, result::Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

const PATH_HOME: &str = "/";
const PATH_METRICS: &str = "/metrics";
const PATH_HEALTH: &str = "/health";
const PATH_READINESS: &str = "/ready";

// starts an HTTP server and exports Prometheus metrics.
pub async fn start(
    log: &Logger,
    bind_addr: &str,
    ftp_addr: SocketAddr,
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
    done: tokio::sync::mpsc::Sender<()>,
) -> Result<(), String> {
    let http_addr: SocketAddr = bind_addr
        .parse()
        .map_err(|e| format!("unable to parse HTTP address {}: {}", bind_addr, e))?;

    let make_svc = make_service_fn(|_socket: &AddrStream| {
        async move {
            // service_fn converts our function into a `Service`
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| async move {
                let handler = HttpHandler { ftp_addr };
                handler.router(req).await
            }))
        }
    });

    let http_server = hyper::Server::bind(&http_addr)
        .serve(make_svc)
        .with_graceful_shutdown(async {
            shutdown.recv().await.ok();
            info!(log, "Shutting down HTTP server");
        });

    info!(log, "Starting HTTP service."; "address" => &http_addr);
    info!(log, "Exposing {} service home.", app::NAME; "path" => PATH_HOME);
    info!(log, "Exposing Prometheus {} exporter endpoint.", app::NAME; "path" => PATH_METRICS);
    info!(log, "Exposing readiness endpoint."; "path" => PATH_READINESS);
    info!(log, "Exposing liveness endpoint."; "path" => PATH_HEALTH);

    if let Err(e) = http_server.await {
        error!(log, "HTTP server error: {}", e)
    }

    info!(log, "HTTP shutdown OK");
    drop(done);
    Ok(())
}

struct HttpHandler {
    pub ftp_addr: SocketAddr,
}

impl HttpHandler {
    async fn router(&self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        let mut response: Response<Body> = Response::new(Body::empty());
        match (req.method(), req.uri().path()) {
            (&Method::GET, PATH_HOME) | (&Method::GET, "/index.html") => {
                *response.body_mut() = self.service_home();
            }
            (&Method::GET, PATH_METRICS) => {
                *response.body_mut() = Body::from(metrics::gather());
            }
            (&Method::GET, PATH_HEALTH) => {
                self.health(&mut response).await;
            }
            (&Method::GET, PATH_READINESS) => {
                *response.status_mut() = StatusCode::OK;
            }
            _ => {
                *response.status_mut() = StatusCode::NOT_FOUND;
            }
        }

        Ok(response)
    }

    fn service_home(&self) -> Body {
        let index_html = include_str!(concat!(env!("PROJ_WEB_DIR"), "/index.html"));
        Body::from(index_html.replace("{{ .AppVersion }}", app::VERSION))
    }

    async fn health(&self, response: &mut Response<Body>) {
        match self.ftp_probe().await {
            Ok(_) => {
                *response.body_mut() = Body::from("<html>OK!</html>");
                *response.status_mut() = StatusCode::OK;
            }
            Err(_e) => {
                // TODO: Log error
                *response.body_mut() = Body::from("<html>Service unavailable!</html>");
                *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
            }
        }
    }

    async fn ftp_probe(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connect_to_addr = if self.ftp_addr.ip().is_unspecified() {
            SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                self.ftp_addr.port(),
            )
        } else {
            self.ftp_addr
        };

        let connection = tokio::net::TcpStream::connect(connect_to_addr).await?;
        let (rx, mut tx) = tokio::io::split(connection);
        let mut reader = tokio::io::BufReader::new(rx);

        // Consume welcome message
        let mut line_buf = String::new();
        reader.read_line(&mut line_buf).await?;

        tx.write_all(b"NOOP\r\n").await?;
        line_buf.clear();
        reader.read_line(&mut line_buf).await?;

        tx.write_all(b"QUIT\r\n").await?;
        line_buf.clear();
        reader.read_line(&mut line_buf).await?;

        Ok(())
    }
}
