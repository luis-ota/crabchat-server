use std::{collections::HashMap, sync::Arc};

use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;

use crate::infra::models::Room;

pub type SharedUsers = Arc<Mutex<HashMap<Uuid, Arc<Mutex<WebSocketStream<TcpStream>>>>>>;
pub type SharedRooms = Arc<Mutex<HashMap<String, Room>>>;
