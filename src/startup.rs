use std::time::Duration;

use axum::{
    Router,
    http::{Request, Response},
    routing::{get, post},
};
use secrecy::ExposeSecret;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use tokio::signal;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::{
    configuration::Settings,
    routes::{health_check, subscribe},
};

pub fn build_app(connection_pool: Pool<Postgres>) -> Router<()> {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(connection_pool)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    let request_id = Uuid::new_v4();
                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri(),
                        headers = ?request.headers(),
                        request_id = %request_id
                    )
                })
                .on_response(
                    |response: &Response<_>, latency: Duration, _span: &tracing::Span| {
                        tracing::info!(
                            status = %response.status(),
                            elapsed_ms = %latency.as_millis(),
                            "response sent"
                        );
                    },
                ),
        )
}

pub fn create_connection_pool(configuration: &Settings) -> Result<Pool<Postgres>, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect_lazy(configuration.database.connection_string().expose_secret())
}

pub async fn get_listener(addr: &str) -> Result<tokio::net::TcpListener, std::io::Error> {
    tokio::net::TcpListener::bind(addr).await
}

pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C handler");
    };

    // #[cfg(not(unix))]
    // let terminate = std::future::pending::<()>();

    tokio::select! {
            _ = ctrl_c => {}
    //         _ = terminate => {}
        }
}
