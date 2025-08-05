use once_cell::sync::Lazy;
use reqwest::Client;
use secrecy::ExposeSecret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use ztp_r::{
    configuration::{DatabaseSettings, get_configuration},
    startup::{build_app, get_listener, shutdown_signal},
    telemetry::{get_subscriber, init_subscriber},
};

// tracing is initialized only once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    // We cannot assign the output of `get_subscriber` to a variable based on the
    // value TEST_LOG` because the sink is part of the type returned by
    // `get_subscriber`, therefore they are not the same type. We could work around
    // it, but this is the most straight-forward way of moving forward.
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

/// Spin up an instance of the application in the background
/// and returns its address
async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    let listener = get_listener("127.0.0.1:0")
        .await
        .expect("Failed to bind to address.");

    let port = listener
        .local_addr()
        .expect("Could not get local addr.")
        .port();

    let ip = listener
        .local_addr()
        .expect("Could not get local addr.")
        .ip()
        .to_string();

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();

    let connection_pool = configure_database(&configuration.database).await;
    let app = build_app(connection_pool.clone());

    let _ = tokio::spawn(async {
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .unwrap()
    });

    let address = format!("http://{ip}:{port}");
    TestApp {
        address,
        db_pool: connection_pool,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // First, connect to Postgres instance (pass '_without_db')
    let mut connection =
        PgConnection::connect(config.connection_string_without_db().expose_secret())
            .await
            .expect("Failed to connect to Postgres.");

    // Create the database
    connection
        .execute(format!(r#"CREATE DATABASE "{}"; "#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // Now connect to the database with 'connection_string' after the database is created
    let connection_pool = PgPool::connect(config.connection_string().expose_secret())
        .await
        .expect("Failed to connect to Postgres.");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database.");

    connection_pool
}

#[tokio::test]
async fn health_check_works() {
    // arrange
    let app = spawn_app().await;
    let client = Client::new();
    let url = format!("{}/health_check", &app.address);

    // act
    let response = client
        .get(url)
        .send()
        .await
        .expect("Failed to execute request.");

    // assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // arrange - app
    let app = spawn_app().await;

    // arrange - client
    let client = Client::new();
    let url = format!("{}/subscriptions", &app.address);
    let body = "name=ardnys&email=ardnys%40gmail.com";

    // act
    let response = client
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // assert
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ardnys@gmail.com");
    assert_eq!(saved.name, "ardnys");
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    // INFO: Actix-web return 400 when data is missing or incorrect.
    // Axum returns 422 by default so we are testing for that here.

    // arrange
    let app = spawn_app().await;
    let client = Client::new();
    let url = format!("{}/subscriptions", &app.address);
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // act
        let response = client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        // assert
        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 422 Unprocessable Content when the payload was {error_message}."
        );
    }
}
