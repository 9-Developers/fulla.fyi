//! Program entry point.

use axum::serve;
use nine_dev_site_lib::{
    server::{self, Server},
    startup,
    telemetry::Telemetry,
};
use std::{env::VarError, net::SocketAddr};
use tokio::net::TcpListener;
use tracing::{info, instrument};
use tracing_subscriber::EnvFilter;

pub use crate::route::*;

mod auth;
mod database;
mod route;
pub mod static_assets;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    startup::load_local_dotenv()?;
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments == ["migrate"] {
        return migrate();
    }
    if !arguments.is_empty() {
        return Err("usage: fulla_fyi [migrate]".into());
    }
    let telemetry = Telemetry::init("fulla.fyi", env!("CARGO_PKG_VERSION"), &["fulla_fyi"])?;
    server::run(telemetry, run)
}

#[tokio::main]
async fn migrate() -> Result<(), Box<dyn std::error::Error>> {
    let log_filter = match startup::optional_env("RUST_LOG")? {
        Some(value) => {
            EnvFilter::try_new(value).map_err(|_| "RUST_LOG must be a valid tracing filter")?
        }
        None => EnvFilter::new("info"),
    };
    tracing_subscriber::fmt()
        .with_env_filter(log_filter)
        .try_init()
        .map_err(|_| "could not initialize tracing")?;

    let pool = database::connect().await?;
    database::MIGRATOR.run(&pool).await?;
    pool.close().await;
    info!("database migrations complete");
    Ok(())
}

#[instrument(skip_all)]
async fn run(server: Server) -> Result<(), Box<dyn std::error::Error>> {
    let pool = database::connect().await?;
    database::check_schema(&pool).await?;
    let secure_cookies = match std::env::var("SESSION_SECURE") {
        Ok(value) if value == "true" => true,
        Ok(value) if value == "false" => false,
        Err(VarError::NotPresent) => true,
        _ => return Err("SESSION_SECURE must be true or false".into()),
    };
    let routes = get_routes(database::AppState { pool: pool.clone() }, secure_cookies).await?;
    let routes = server.requests.instrument(routes, &static_assets::CATALOG);
    let listener = TcpListener::bind(server.address).await?;
    tokio::spawn(auth::cleanup(pool));
    info!(address = %listener.local_addr()?, "listening");

    serve(
        listener,
        routes.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(server.shutdown)
    .await?;

    Ok(())
}
