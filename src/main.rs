mod infra;

use crate::infra::enums::{Action, IncomingMessage, ResType, ServerError};
use crate::infra::models::{CreateRoom, Room, ServerMessage, User, UserMessage};
use anyhow::Result;
use chrono::Utc;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{FutureExt, SinkExt, StreamExt, TryFutureExt};
use infra::models::AcessRoom;
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
                    let result = process_message(
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
) -> Result<(), ServerError> {
    match msg {
        IncomingMessage::User(user_data) => {
            info!("Usuário se identificou: {:?}", user_data);
            register_user(current_user, user_data, user_id, users, ws_sender).await?;
            send_rooms(rooms, ws_sender).await?;
        }
        IncomingMessage::CreateRoom(room_data) => {
            create_room(current_user, room_data, rooms, ws_sender).await?;
        }
        IncomingMessage::AcessRoom(room_access) => {
            acess_room(current_user, room_access, rooms, users, ws_sender).await?;
        }
        IncomingMessage::LeaveRoom(room_leave) => {
            todo!()
        }

        IncomingMessage::DeleteRoom(delete_room) => {
            todo!()
        }
        IncomingMessage::UserMessage(mut user_message) => {
            todo!()
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

async fn check_session(current_user: &mut Option<User>) -> Result<(), ServerError> {
    current_user
        .as_ref()
        .ok_or(ServerError::UnauthorizedMethod)?;
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
    current_user: &mut Option<User>,

    room_data: CreateRoom,
    rooms: &SharedRooms,
    ws_sender: &Arc<Mutex<WebSocketStream<TcpStream>>>,
) -> Result<(), ServerError> {
    check_session(current_user).await?;
    info!("Received room: {:?}", room_data);

    let mut room_users = HashMap::new();

    room_users.insert(
        current_user.as_ref().unwrap().uuid.to_string(),
        current_user.as_ref().unwrap().to_owned(),
    );

    rooms.lock().await.insert(
        room_data.base_info.code.clone(),
        Room {
            info: room_data.clone(),
            messages: Vec::new(),
            users: room_users,
        },
    );

    ws_sender
        .lock()
        .await
        .send(Message::from(
            serde_json::to_string(&ServerMessage {
                for_action: Action::CreateRoom,
                res_type: ResType::Success,
                message: "Room created successfully".to_owned(),
            })
            .map_err(ServerError::Serialization)?,
        ))
        .await
        .map_err(ServerError::WebSocket)?;
    Ok(())
}

async fn get_room_users(rooms: &SharedRooms, room_code: &String) -> Result<Vec<User>, ServerError> {
    let rooms_lock = rooms.lock().await;

    let room = rooms_lock
        .get(room_code)
        .ok_or(ServerError::RoomNotFound(room_code.to_owned()))?;

    Ok(room.users.clone().into_values().collect())
}

async fn acess_room(
    current_user: &mut Option<User>,
    acess_room: AcessRoom,
    rooms: &SharedRooms,
    users: &SharedUsers,
    ws_sender: &Arc<Mutex<WebSocketStream<TcpStream>>>,
) -> Result<(), ServerError> {
    check_session(current_user).await?;
    info!(
        "User {:?} required acess to room {:?}",
        current_user, acess_room.code
    );

    let mut rooms_lock = rooms.lock().await;
    let room_code = &acess_room.code;

    let room = rooms_lock
        .get_mut(room_code)
        .ok_or_else(|| ServerError::RoomNotFound(room_code.to_owned()))?;

    if room.info.password.eq(&acess_room.password) {
        let user = current_user.as_ref().unwrap();
        room.users.insert(user.uuid.clone(), user.to_owned());

        let public_info = room.public_info();

        let public_room_value =
            serde_json::to_string(&public_info).map_err(ServerError::Serialization)?;

        let message_json = serde_json::to_string(&ServerMessage {
            for_action: Action::AcessRoom,
            res_type: ResType::Success,
            message: public_room_value,
        })
        .map_err(ServerError::Serialization)?;

        ws_sender
            .lock()
            .await
            .send(Message::from(message_json))
            .map_err(ServerError::WebSocket)
            .await?;

        let user_message = UserMessage {
            user: Some(user.to_owned()),
            message: "joined room".to_owned(),
            datetime: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            room: acess_room.code.to_owned(),
        };

        broadcast(
            users,
            get_room_users(rooms, &acess_room.code).await?,
            serde_json::to_string(&user_message).map_err(ServerError::Serialization)?,
        )
        .await?;
    } else {
        ws_sender
            .lock()
            .await
            .send(Message::from(
                serde_json::to_string(&ServerMessage {
                    for_action: Action::AcessRoom,
                    res_type: ResType::ServerError,
                    message: "Incorrect Password".to_string(),
                })
                .map_err(ServerError::Serialization)?,
            ))
            .await
            .map_err(ServerError::WebSocket)?;
    }
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

// OLD CODE FOR REFERENCE PLS IGNOREEE
// match serde_json::from_str::<IncomingMessage>(&text) {
//     Ok(IncomingMessage::User(user)) => {
//         println!("WebSocket connection established for user: {:?}", user);

//         ref_user.clone().name = user.name.clone();
//         users_ws_streams
//             .lock()
//             .await
//             .insert(uuid, (ref_user.clone(), ws_stream.clone()));
//         let avaliable_rooms: Vec<_> = rooms
//             .lock()
//             .await
//             .values()
//             .clone()
//             .filter(|r| r.info.public)
//             .map(|r| CreateRoom {
//                 base_info: r.info.base_info.clone(),
//                 password: None,
//                 public: r.info.public,
//             })
//             .collect();

//         let avaliable_rooms_json =
//             serde_json::to_string(&avaliable_rooms).expect("serialize rooms");

//         ws_stream
//             .lock()
//             .await
//             .send(Message::from(avaliable_rooms_json))
//             .await
//             .expect("send initials info");
//     }

//     Ok(IncomingMessage::CreateRoom(room)) => {
//         println!("Received room: {:?}", room);
//         let mut users = HashMap::new();
//         users.insert(ref_user.uuid.clone(), ref_user.clone());
//         let new_room = Room {
//             info: room.clone(),
//             messages: Vec::new(),
//             users: users.clone(),
//         };

//         rooms
//             .lock()
//             .await
//             .insert(room.base_info.code.clone(), new_room);

//         ws_stream
//             .lock()
//             .await
//             .send(Message::from(
//                 serde_json::to_string(&ServerMessage {
//                     for_action: Action::CreateRoom,
//                     res_type: ResType::Success,
//                     message: "Room created successfully".to_string(),
//                 })
//                 .expect("serialize rooms"),
//             ))
//             .await
//             .expect("Fails to send initils info");
//     }

//     Ok(IncomingMessage::AcessRoom(room)) => {
//         println!("User {:?} required acess to {:?}", ref_user, room.code);

//         let mut acess_room: Room = rooms.lock().await.get(&room.code).unwrap().clone();
//         if acess_room.info.password.eq(&room.password) {
//             acess_room
//                 .users
//                 .insert(ref_user.uuid.clone(), ref_user.clone());

//             ws_stream
//                 .lock()
//                 .await
//                 .send(Message::from(
//                     serde_json::to_string(&ServerMessage {
//                         for_action: Action::AcessRoom,
//                         res_type: ResType::Success,
//                         message: serde_json::to_string(&Room {
//                             users: acess_room.users.clone(),
//                             messages: acess_room.messages.clone(),
//                             info: CreateRoom {
//                                 base_info: acess_room.info.base_info.clone(),
//                                 password: None,
//                                 public: acess_room.info.public,
//                             },
//                         })
//                         .unwrap(),
//                     })
//                     .unwrap(),
//                 ))
//                 .await
//                 .expect("Fails to send server res");
//         } else {
//             ws_stream
//                 .lock()
//                 .await
//                 .send(Message::from(
//                     serde_json::to_string(&ServerMessage {
//                         for_action: Action::AcessRoom,
//                         res_type: ResType::Error,
//                         message: "Incorrect Password".to_string(),
//                     })
//                     .expect("serialize res"),
//                 ))
//                 .await
//                 .expect("Fails to send server res");
//         }
//     }
//     Ok(IncomingMessage::LeaveRoom(room)) => {
//         let mut leave_room: Room = rooms.lock().await.get(&room.code).unwrap().clone();
//         leave_room.users.remove(&ref_user.uuid);

//         broadcast(
//             users_ws_streams.clone(),
//             leave_room.users.into_values().collect(),
//             serde_json::to_string(&UserMessage {
//                 user: Some(ref_user.clone()),
//                 message: "*Leaving Room*".to_string(),
//                 datetime: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
//                 room: room.code,
//             })
//             .expect("serialize user message"),
//         )
//         .await;
//     }
//     Ok(IncomingMessage::UserMessage(mut user_message)) => {
//         user_message.user = Some(ref_user.clone());
//         user_message.datetime = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

//         let room: Room = rooms.lock().await.get(&user_message.room).unwrap().clone();
//         room.clone().messages.push(user_message.clone());
//         let users_to_bc = room.users.into_values().collect();
//         broadcast(
//             users_ws_streams.clone(),
//             users_to_bc,
//             serde_json::to_string(&user_message.clone())
//                 .expect("serialize user message"),
//         )
//         .await;
//     }

//     Ok(IncomingMessage::DeleteRoom(delete_room)) => {
//         let deleted_room: Room = rooms.lock().await.remove(&delete_room.room).unwrap();
//         broadcast(
//             users_ws_streams.clone(),
//             deleted_room.users.into_values().collect(),
//             serde_json::to_string(&ServerMessage {
//                 for_action: Action::DeleteRoom,
//                 res_type: ResType::Success,
//                 message: serde_json::to_string(&deleted_room.info.base_info.clone())
//                     .expect("serialize room"),
//             })
//             .expect("serialize user message"),
//         )
//         .await;
//     }

//     _ => {
//         println!("Message does not follow any pattern: {text}");
//         ws_stream
//             .lock()
//             .await
//             .send(Message::from(
//                 serde_json::to_string(&ServerMessage {
//                     for_action: Action::InvalidRequest,
//                     res_type: ResType::Error,
//                     message: "Message does not follow any pattern: {text}".to_string(),
//                 })
//                 .expect("serialize res"),
//             ))
//             .await
//             .expect("Fails to send server res");
//     }
// }
