use ztp_r::{
    configuration::get_configuration,
    startup::{build_app, create_connection_pool, get_listener, shutdown_signal},
};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt::init();

    // TODO: make this clean as well
    let configuration = get_configuration().expect("Failed to read configuration.");

    let connection_pool = create_connection_pool(&configuration)
        .await
        .expect("Failed to connect to database.");

    let listener = get_listener("127.0.0.1:8000").await?;
    let app = build_app(connection_pool);

    println!("Listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}
