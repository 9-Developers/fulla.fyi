//! Program entry point.

use axum::serve;
use nine_dev_site_lib::startup;
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
    run()
}

#[tokio::main]
#[instrument]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
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

    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if !arguments.is_empty() && arguments != ["migrate"] {
        return Err("usage: fulla_fyi [migrate]".into());
    }
    let pool = database::connect().await?;
    if arguments == ["migrate"] {
        database::MIGRATOR.run(&pool).await?;
        pool.close().await;
        info!("database migrations complete");
        return Ok(());
    }
    let port = startup::port()?;
    database::check_schema(&pool).await?;
    let secure_cookies = match std::env::var("SESSION_SECURE") {
        Ok(value) if value == "true" => true,
        Ok(value) if value == "false" => false,
        Err(VarError::NotPresent) => true,
        _ => return Err("SESSION_SECURE must be true or false".into()),
    };
    let routes = get_routes(database::AppState { pool: pool.clone() }, secure_cookies).await?;
    tokio::spawn(auth::cleanup(pool));
    let address = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&address).await?;
    info!(address = %listener.local_addr()?, "listening");

    serve(
        listener,
        routes.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
