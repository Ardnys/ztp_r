use std::time::Duration;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use tokio::signal;

use crate::{
    configuration::Settings,
    routes::{health_check, subscribe},
};

pub fn build_app(connection_pool: Pool<Postgres>) -> Router<()> {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(connection_pool)
}

pub async fn create_connection_pool(
    configuration: &Settings,
) -> Result<Pool<Postgres>, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&configuration.database.connection_string())
        .await
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
