mod infra;

use crate::infra::enums::{Action, IncomingMessage, ResType, ServerError};
use crate::infra::models::{CreateRoom, Room, ServerMessage, User, UserMessage};
use anyhow::Result;
use chrono::Utc;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{WebSocketStream, accept_async};
use tracing::{error, info, instrument, warn};
use tracing_subscriber;
use uuid::Uuid;

type SharedUsers = Arc<Mutex<HashMap<Uuid, Arc<Mutex<WebSocketStream<TcpStream>>>>>>;
type SharedRooms = Arc<Mutex<HashMap<String, Room>>>;

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

#[instrument(skip_all)]
async fn handle_connection(
    stream: TcpStream,
    users: SharedUsers,
    rooms: Arc<Mutex<HashMap<String, Room>>>,
) -> Result<()> {
    let ws_stream = Arc::new(Mutex::new(accept_async(stream).await?));
    info!("Conexão WebSocket estabelecida com sucesso.");

    let user_id = Uuid::new_v4();
    let mut current_user: Option<User> = None;

    while let Some(msg) = ws_stream.lock().await.next().await {
        if let Ok(Message::Text(text)) = msg {
            match serde_json::from_str::<IncomingMessage>(&text) {
                Ok(incoming_message) => {
                    let result = process_message(
                        incoming_message,
                        &user_id,
                        &mut current_user,
                        &users,
                        &rooms,
                        &Arc::clone(&ws_stream),
                    )
                    .await;
                }
                Err(e) => {
                    warn!("Falha ao desserializar a mensagem: {}. Texto: {}", e, text);
                }
            }
        }
    }

    Ok(())
}

#[instrument(skip(users, rooms, ws_sender))]
async fn process_message(
    msg: IncomingMessage,
    user_id: &Uuid,
    current_user: &mut Option<User>,
    users: &SharedUsers,
    rooms: &SharedRooms,
    ws_sender: &Arc<Mutex<WebSocketStream<TcpStream>>>,
) -> Result<()> {
    match msg {
        IncomingMessage::User(user_data) => {
            info!("Usuário se identificou: {:?}", user_data);

            register_user(user_data, user_id, current_user, users, ws_sender).await;

            send_rooms(rooms, ws_sender).await;
        }
        IncomingMessage::CreateRoom(room_data) => {
            todo!()
        }
        IncomingMessage::AcessRoom(room_access) => {
            todo!()
        }
        IncomingMessage::LeaveRoom(room_leave) => {
            todo!()
        }
        IncomingMessage::UserMessage(mut user_message) => {
            todo!()
        }
        IncomingMessage::DeleteRoom(delete_room) => {
            todo!()
        }
    }
    Ok(())
}

async fn register_user(
    user_data: User,
    user_id: &Uuid,
    current_user: &mut Option<User>,
    users: &SharedUsers,
    ws_sender: &Arc<Mutex<WebSocketStream<TcpStream>>>,
) -> Result<(), ServerError> {
    let new_user = User {
        uuid: user_id.to_string(),
        ..user_data
    };

    users
        .lock()
        .await
        .insert(user_id.to_owned(), Arc::clone(ws_sender));

    *current_user = Some(new_user);
    Ok(())
}

async fn send_rooms(rooms: &SharedRooms, ws_sender: &Arc<Mutex<WebSocketStream<TcpStream>>>) {
    let avaliable_rooms: Vec<CreateRoom> = rooms
        .lock()
        .await
        .values()
        .filter(|r| r.info.public)
        .map(|r| CreateRoom {
            base_info: r.info.base_info.to_owned(),
            password: None,
            public: r.info.public,
        })
        .collect();

    let avaliable_rooms_json = serde_json::to_string(&avaliable_rooms).expect("serialize rooms");

    ws_sender
        .lock()
        .await
        .send(Message::from(avaliable_rooms_json))
        .await
        .expect("send initials info");
}

async fn broadcast(users_ws_streams: SharedUsers, users_to_bc: Vec<User>, message: String) {
    for user in users_to_bc {
        todo!()
        // let ws_stream = users_ws_streams
        //     .lock()
        //     .await
        //     .get(&Uuid::parse_str(&user.uuid).unwrap())
        //     .unwrap()
        //     .1
        //     .clone();

        // ws_stream
        //     .lock()
        //     .await
        //     .send(Message::from(message.clone()))
        //     .await
        //     .expect("broadcast message");
    }
}
