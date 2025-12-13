//! Metrics HTTP Exporter
//! 
//! Provides HTTP endpoint for Prometheus scraping.

use std::net::SocketAddr;

use hyper::{Body, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};

/// Metrics exporter server
pub struct MetricsExporter {
    addr: SocketAddr,
}

impl MetricsExporter {
    /// Create a new metrics exporter
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
    
    /// Start the metrics HTTP server
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        let make_svc = make_service_fn(|_conn| async {
            Ok::<_, hyper::Error>(service_fn(handle_metrics_request))
        });
        
        let server = Server::bind(&self.addr).serve(make_svc);
        
        tracing::info!("Metrics server listening on http://{}/metrics", self.addr);
        
        server.await?;
        
        Ok(())
    }
}

/// Handle metrics HTTP request
async fn handle_metrics_request(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match req.uri().path() {
        "/metrics" => {
            match crate::prometheus::export_metrics() {
                Ok(metrics) => {
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "text/plain; version=0.0.4")
                        .body(Body::from(metrics))
                        .unwrap())
                }
                Err(e) => {
                    tracing::error!("Failed to export metrics: {}", e);
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from(format!("Error: {}", e)))
                        .unwrap())
                }
            }
        }
        "/health" => {
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from("OK"))
                .unwrap())
        }
        _ => {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))
                .unwrap())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_export() {
        let metrics = crate::prometheus::export_metrics();
        assert!(metrics.is_ok());
        
        let output = metrics.unwrap();
        assert!(output.contains("jsp_connections_total"));
        assert!(output.contains("jsp_bytes_sent_total"));
    }
}
