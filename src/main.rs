mod infra;

use crate::infra::enums::{Action, IncomingMessage, ResType, ServerError};
use crate::infra::models::{CreateRoom, Room, ServerMessage, ToJson, User, UserMessage};
use anyhow::Result;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use infra::models::{AcessRoom, DeleteRoom, LeaveRoom};
use std::collections::HashMap;
use std::str::FromStr;
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
) -> Result<(), ServerError> {
    let ws_stream = Arc::new(Mutex::new(accept_async(stream).await?));
    info!("Conexão WebSocket estabelecida com sucesso.");

    let user_id = Uuid::new_v4();
    let mut current_user: Option<User> = None;

    while let Some(msg) = ws_stream.lock().await.next().await {
        if let Ok(Message::Text(text)) = msg {
            match serde_json::from_str::<IncomingMessage>(&text) {
                Ok(incoming_message) => {
                    process_message(
                        incoming_message,
                        &user_id,
                        &mut current_user,
                        &users,
                        &rooms,
                        &Arc::clone(&ws_stream),
                    )
                    .await?;
                }
                Err(e) => {
                    error!("erro ao desserializar a mensagem: {}. Texto: {}", e, text);
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
) -> Result<(), ServerError> {
    let user = check_session(current_user).await?;

    match msg {
        IncomingMessage::User(user_data) => {
            register_user(current_user, user_data, user_id, users, ws_sender).await?;
            send_rooms(rooms, ws_sender).await?;
        }
        IncomingMessage::CreateRoom(room_data) => {
            create_room(user, room_data, rooms, ws_sender).await?;
        }
        IncomingMessage::AcessRoom(room_access) => {
            acess_room(user, room_access, rooms, users, ws_sender).await?;
        }
        IncomingMessage::LeaveRoom(room_leave) => {
            leave_room(user, rooms, users, &room_leave).await?;
        }

        IncomingMessage::DeleteRoom(room_delete) => {
            delete_room(user, ws_sender, users, rooms, &room_delete).await?;
        }
        IncomingMessage::UserMessage(user_message) => {
            broadcast(
                users,
                get_room_users(rooms, &user_message.room).await?,
                user_message.to_json()?,
            )
            .await?
        }
    }
    Ok(())
}

async fn register_user(
    current_user: &mut Option<User>,

    user_data: User,
    user_id: &Uuid,
    users: &SharedUsers,
    ws_sender: &Arc<Mutex<WebSocketStream<TcpStream>>>,
) -> Result<(), ServerError> {
    info!("Usuário se identificou: {:?}", user_data);

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

async fn check_session(current_user: &mut Option<User>) -> Result<&User, ServerError> {
    current_user.as_ref().ok_or(ServerError::Unauthorized)?;
    Ok(current_user.as_ref().unwrap())
}

async fn server_response(
    ws_sender: &Arc<Mutex<WebSocketStream<TcpStream>>>,
    for_action: Action,
    res_type: ResType,
    message: String,
) -> Result<(), ServerError> {
    ws_sender
        .lock()
        .await
        .send(Message::from(
            ServerMessage {
                for_action,
                res_type,
                message,
            }
            .to_json()?,
        ))
        .await
        .map_err(ServerError::WebSocket)?;
    Ok(())
}

async fn send_rooms(
    rooms: &SharedRooms,
    ws_sender: &Arc<Mutex<WebSocketStream<TcpStream>>>,
) -> Result<(), ServerError> {
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

    let avaliable_rooms_json =
        serde_json::to_string(&avaliable_rooms).map_err(ServerError::Serialization)?;

    ws_sender
        .lock()
        .await
        .send(Message::from(avaliable_rooms_json))
        .await
        .map_err(ServerError::WebSocket)?;

    Ok(())
}

async fn create_room(
    user: &User,

    room_data: CreateRoom,
    rooms: &SharedRooms,
    ws_sender: &Arc<Mutex<WebSocketStream<TcpStream>>>,
) -> Result<(), ServerError> {
    info!("Received room: {:?}", room_data);

    let mut room_users = HashMap::new();

    room_users.insert(user.uuid.to_string(), user.to_owned());

    rooms.lock().await.insert(
        room_data.base_info.code.clone(),
        Room {
            info: room_data.clone(),
            messages: Vec::new(),
            users: room_users,
        },
    );
    server_response(
        ws_sender,
        Action::CreateRoom,
        ResType::Success,
        "room created successfully".to_owned(),
    )
    .await?;

    Ok(())
}

async fn delete_room(
    user: &User,
    ws_sender: &Arc<Mutex<WebSocketStream<TcpStream>>>,
    users: &SharedUsers,
    rooms: &SharedRooms,
    room_delete: &DeleteRoom,
) -> Result<(), ServerError> {
    let room = get_room(rooms, &room_delete.code).await?;
    if room.info.base_info.created_by == *user {
        if room.info.password == room_delete.password {
            rooms.lock().await.remove(&room_delete.code);
            broadcast(
                users,
                get_room_users(rooms, &room_delete.code).await?,
                "this room was deleted".to_string(),
            )
            .await?;
        } else {
            server_response(
                ws_sender,
                Action::DeleteRoom,
                ResType::ServerError,
                "incorrect password".to_string(),
            )
            .await?;
        }
    } else {
        server_response(
            ws_sender,
            Action::DeleteRoom,
            ResType::ServerError,
            "rooms can only be deleted by its creator".to_string(),
        )
        .await?;
    }

    Ok(())
}

async fn get_room_users(rooms: &SharedRooms, room_code: &String) -> Result<Vec<User>, ServerError> {
    let rooms_lock = rooms.lock().await;

    let room = rooms_lock
        .get(room_code)
        .ok_or(ServerError::RoomNotFound(room_code.to_owned()))?;

    Ok(room.users.clone().into_values().collect())
}

async fn get_room(rooms: &SharedRooms, code: &String) -> Result<Room, ServerError> {
    let rooms_lock = rooms.lock().await;

    let room = rooms_lock
        .get(code)
        .ok_or(ServerError::RoomNotFound(code.to_owned()))?;
    Ok(room.to_owned())
}

async fn acess_room(
    user: &User,
    acess_room: AcessRoom,
    rooms: &SharedRooms,
    users: &SharedUsers,
    ws_sender: &Arc<Mutex<WebSocketStream<TcpStream>>>,
) -> Result<(), ServerError> {
    info!(
        "User {:?} required acess to room {:?}",
        user, acess_room.code
    );

    let mut room = get_room(rooms, &acess_room.code).await?;

    if room.info.password.eq(&acess_room.password) {
        room.users.insert(user.uuid.clone(), user.to_owned());

        let public_info = room.public_info();

        server_response(
            ws_sender,
            Action::AcessRoom,
            ResType::Success,
            public_info.to_json()?,
        )
        .await?;

        let user_message = UserMessage {
            user: Some(user.to_owned()),
            message: "joined room".to_string(),
            datetime: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            room: acess_room.code.to_owned(),
        };

        broadcast(
            users,
            get_room_users(rooms, &acess_room.code).await?,
            user_message.to_json()?,
        )
        .await?;
    } else {
        server_response(
            ws_sender,
            Action::AcessRoom,
            ResType::ServerError,
            "incorrect password".to_string(),
        )
        .await?;
    }
    Ok(())
}

async fn leave_room(
    user: &User,
    rooms: &SharedRooms,
    users: &SharedUsers,
    leave_room: &LeaveRoom,
) -> Result<(), ServerError> {
    let mut room = get_room(rooms, &leave_room.code).await?;

    room.users.remove(&user.uuid);
    let message = UserMessage {
        user: Some(user.to_owned()),
        message: "leaved room".to_string(),
        datetime: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        room: leave_room.code.to_owned(),
    };
    broadcast(
        users,
        get_room_users(rooms, &leave_room.code).await?,
        message.to_json()?,
    )
    .await?;

    Ok(())
}

async fn broadcast(
    users_ws_streams: &SharedUsers,
    users_to_bc: Vec<User>,
    message: String,
) -> Result<(), ServerError> {
    let users_sw_lock = users_ws_streams.lock().await;

    for user in users_to_bc {
        let uuid = Uuid::from_str(&user.uuid).map_err(|_| ServerError::InvalidUuid(user.uuid))?;
        let ws_sender = users_sw_lock
            .get(&uuid)
            .ok_or(ServerError::UserNotFound(user.name))?;

        ws_sender
            .lock()
            .await
            .send(Message::from(message.to_owned()))
            .await
            .map_err(ServerError::WebSocket)?;
    }

    Ok(())
}
