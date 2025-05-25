mod auth;
mod infra;
mod message;
mod room;
mod types;
mod websocket;
use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};
use tracing::info;
use types::{SharedRooms, SharedUsers};
use websocket::handle_connection;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let users = SharedUsers::new(Mutex::new(HashMap::new()));
    let rooms = SharedRooms::new(Mutex::new(HashMap::new()));

    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(&addr).await?;
    info!("WebSocket server started on ws://{}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let users = Arc::clone(&users);
        let rooms = Arc::clone(&rooms);

        tokio::spawn(handle_connection(stream, users, rooms));
    }

    Ok(())
}
