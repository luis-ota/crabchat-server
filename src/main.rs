mod args;
mod auth;
mod infra;
mod message;
mod room;
mod types;
mod websocket;
use anyhow::Result;
use args::parse_args;
use std::{collections::HashMap, env, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};
use tracing::info;
use types::{SharedRooms, SharedUsers};
use websocket::handle_connection;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let parsed_args = match parse_args(args).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    tracing_subscriber::fmt::init();

    let users = SharedUsers::new(Mutex::new(HashMap::new()));
    let rooms = SharedRooms::new(Mutex::new(HashMap::new()));

    let addr = "0.0.0.0";
    let port = &parsed_args.get("port").unwrap();
    let listener = TcpListener::bind(&format!("{addr}:{port}")).await?;
    info!("WebSocket server started on ws://{addr}:{port}");

    while let Ok((stream, _)) = listener.accept().await {
        let users = Arc::clone(&users);
        let rooms = Arc::clone(&rooms);

        tokio::spawn(handle_connection(stream, users, rooms));
    }

    Ok(())
}
