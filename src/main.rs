//! Program entry point.

use axum::serve;
use dotenvy::dotenv;
use std::{env::VarError, net::SocketAddr, path::Path};
use tokio::net::TcpListener;
use tracing::{info, instrument};
use tracing_subscriber::EnvFilter;

pub use crate::route::*;

mod auth;
mod database;
mod route;
pub mod static_assets;

#[instrument]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if Path::new(".env").exists() {
        dotenv().map_err(|_| "could not load local .env file")?;
    }
    let log_filter = match std::env::var("RUST_LOG") {
        Ok(value) => {
            EnvFilter::try_new(value).map_err(|_| "RUST_LOG must be a valid tracing filter")?
        }
        Err(VarError::NotPresent) => EnvFilter::new("info"),
        Err(VarError::NotUnicode(_)) => return Err("RUST_LOG must contain valid Unicode".into()),
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
    database::check_schema(&pool).await?;
    let secure_cookies = match std::env::var("SESSION_SECURE") {
        Ok(value) if value == "true" => true,
        Ok(value) if value == "false" => false,
        Err(VarError::NotPresent) => true,
        _ => return Err("SESSION_SECURE must be true or false".into()),
    };
    let routes = get_routes(database::AppState { pool: pool.clone() }, secure_cookies).await?;
    tokio::spawn(auth::cleanup(pool));
    let port = configured_port()?;
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

fn configured_port() -> Result<u16, Box<dyn std::error::Error>> {
    let value = match std::env::var("PORT") {
        Ok(value) => Some(value),
        Err(VarError::NotPresent) => None,
        Err(VarError::NotUnicode(_)) => return Err("PORT must contain valid Unicode".into()),
    };
    parse_port(value.as_deref()).map_err(Into::into)
}

fn parse_port(value: Option<&str>) -> Result<u16, &'static str> {
    value.map_or(Ok(3000), |value| {
        value
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .ok_or("PORT must be an integer from 1 to 65535")
    })
}

#[cfg(test)]
mod startup_tests {
    use super::parse_port;

    #[test]
    fn port_defaults_and_rejects_invalid_supplied_values() {
        assert_eq!(parse_port(None), Ok(3000));
        assert_eq!(parse_port(Some("4020")), Ok(4020));
        for value in ["", "0", "abc", "65536"] {
            assert!(parse_port(Some(value)).is_err());
        }
    }
}
