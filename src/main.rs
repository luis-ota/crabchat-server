mod infra;

use crate::infra::enums::{Action, IncomingMessage, ResType};
use crate::infra::models::{BaseRoomInfo, Room, ServerResponse, User, UserMessage};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use infra::models::CreateRoom;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{WebSocketStream, accept_async};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let users = Arc::new(Mutex::new(HashMap::<
        Uuid,
        (User, Arc<Mutex<WebSocketStream<TcpStream>>>),
    >::new()));
    let rooms = Arc::new(Mutex::new(HashMap::<String, Room>::new()));

    let addr = String::from("0.0.0.0:8080");
    let listener = TcpListener::bind(&addr).await?;
    println!("WebSocket server started on ws://{}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let users = users.clone();
        let rooms = rooms.clone();
        tokio::spawn(handle_connection(stream, users, rooms));
    }

    Ok(())
}

async fn handle_connection(
    stream: TcpStream,
    users: Arc<Mutex<HashMap<Uuid, (User, Arc<Mutex<WebSocketStream<TcpStream>>>)>>>,
    rooms: Arc<Mutex<HashMap<String, Room>>>,
) -> Result<()> {
    let ws_stream = Arc::new(Mutex::new(accept_async(stream).await?));
    let uuid = Uuid::new_v4();
    let ref_user: User = User {
        name: String::new(),
        uuid: Some(uuid.to_string()),
    };

    while let Some(msg) = ws_stream.lock().await.next().await {
        if let Ok(Message::Text(text)) = msg {
            match serde_json::from_str::<IncomingMessage>(&text) {
                Ok(IncomingMessage::User(user)) => {
                    println!("WebSocket connection established for user: {:?}", user);
                    ref_user.clone().name = user.name.clone();
                    users
                        .lock()
                        .await
                        .insert(uuid.clone(), (user, ws_stream.clone()));
                    let avaliable_rooms: Vec<_> = rooms
                        .lock()
                        .await
                        .values()
                        .clone()
                        .filter(|r| r.info.public)
                        .map(|r| r.info.clone())
                        .collect();

                    let json = serde_json::to_string(&avaliable_rooms).expect("serialize rooms");

                    ws_stream
                        .lock()
                        .await
                        .send(Message::from(json))
                        .await
                        .expect("send initials info");
                }

                Ok(IncomingMessage::CreateRoom(room)) => {
                    println!("Received room: {:?}", room);
                    let new_room = Room {
                        info: room.clone(),
                        messages: Vec::new(),
                        users: vec![room.clone().base_info.created_by.clone()],
                    };

                    rooms
                        .lock()
                        .await
                        .insert(room.base_info.code.clone(), new_room);

                    let res = ServerResponse {
                        for_action: Action::CreateRoom,
                        res_type: ResType::Success,
                        message: "Room created successfully".to_string(),
                    };

                    ws_stream
                        .lock()
                        .await
                        .send(Message::from(
                            serde_json::to_string(&res).expect("serialize rooms"),
                        ))
                        .await
                        .expect("Fails to send initils info");
                }

                Ok(IncomingMessage::AcessRoom(room)) => {
                    println!("User {:?} required acess to {:?}", ref_user, room.code);

                    let mut acess_room: Room = rooms.lock().await.get(&room.code).unwrap().clone();
                    if acess_room.info.password.eq(&room.password) {
                        acess_room.users.push(ref_user.clone());

                        ws_stream
                            .lock()
                            .await
                            .send(Message::from(
                                serde_json::to_string(&ServerResponse {
                                    for_action: Action::AcessRoom,
                                    res_type: ResType::Success,
                                    message: serde_json::to_string(&Room {
                                        users: acess_room.users.clone(),
                                        messages: acess_room.messages.clone(),
                                        info: CreateRoom {
                                            base_info: acess_room.info.base_info.clone(),
                                            password: None,
                                            public: acess_room.info.public.clone(),
                                        },
                                    })
                                    .unwrap(),
                                })
                                .unwrap(),
                            ))
                            .await
                            .expect("Fails to send server res");
                    } else {
                        ws_stream
                            .lock()
                            .await
                            .send(Message::from(
                                serde_json::to_string(&ServerResponse {
                                    for_action: Action::AcessRoom,
                                    res_type: ResType::Error,
                                    message: "Incorrect Password".to_string(),
                                })
                                .expect("serialize res"),
                            ))
                            .await
                            .expect("Fails to send server res");
                    }
                }

                Ok(IncomingMessage::UserMessage(mut user_message)) => {
                    user_message.user = Some(ref_user.clone());
                    let room = rooms.lock().await.get(&user_message.room).unwrap().clone();
                }

                Err(e) => {
                    let res = ServerResponse {
                        for_action: Action::Connect,
                        res_type: ResType::Error,
                        message: e.to_string(),
                    };

                    ws_stream
                        .lock()
                        .await
                        .send(Message::from(
                            serde_json::to_string(&res).expect("serialize rooms"),
                        ))
                        .await
                        .expect("Sendo error message");
                }
            }
        }
    }

    Ok(())
}

async fn broadcast(
    users: Arc<Mutex<HashMap<Uuid, (User, Arc<Mutex<WebSocketStream<TcpStream>>>)>>>,
    room: Arc<Mutex<Room>>,
    message: UserMessage,
) {
    let users_in_room = users.lock().await;
}
