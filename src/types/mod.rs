use std::{collections::HashMap, sync::Arc};

use futures_util::stream::SplitSink;
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;
use uuid::Uuid;

use crate::infra::models::Room;

pub type SharedUsers =
    Arc<Mutex<HashMap<Uuid, Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>>>>;
pub type SharedRooms = Arc<Mutex<HashMap<String, Room>>>;
