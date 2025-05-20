mod infra;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, WebSocketStream};
use tokio_tungstenite::tungstenite::protocol::Message;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use uuid::Uuid;
use crate::infra::models::{BaseRoomInfo, Room, ServerResponse, User, UserMessage};
use crate::infra::enums::{Action, IncomingMessage, ResType};

#[tokio::main]
async fn main() -> Result<()> {
    let users =  Arc::new(Mutex::new(HashMap::<Uuid, (User, Arc<Mutex<WebSocketStream<TcpStream>>>)>::new()));
    let rooms =  Arc::new(Mutex::new(HashMap::<String, Room>::new()));

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
    let mut ref_user = User {name: String::new(), uuid: Some(uuid.to_string())};
    
    while let Some(msg) = ws_stream.lock().await.next().await {
        if let Ok(Message::Text(text)) = msg {
            match serde_json::from_str::<IncomingMessage>(&text) {
                Ok(IncomingMessage::User(user)) => {
                    println!("WebSocket connection established for user: {:?}", user);
                    ref_user.clone().name = user.name.clone();
                    users.lock().await.insert(
                        uuid.clone(),
                        (user, ws_stream.clone())
                    );
                    let avaliable_rooms: Vec<_> = rooms.lock().await.values().clone().filter(|r| r.info.public).map(
                        | r| r.info.clone()).collect();

                    let json = serde_json::to_string(&avaliable_rooms).expect("serialize rooms");

                    ws_stream.lock().await.send(Message::from(json)).await.expect("send initials info");
                }

                Ok(IncomingMessage::CreateRoom(room)) => {
                    println!("Received room: {:?}", room);
                    let new_room = Room {
                        info: room.clone(),
                        messages: Vec::new(),
                        users: vec![room.clone().base_info.created_by.clone()],
                    };
                    
                    rooms.lock().await.insert(room.base_info.code.clone(), new_room);
                    
                    let res = ServerResponse {
                        for_action: Action::CreateRoom,
                        res_type: ResType::Success,
                        message: "Room created successfully".to_string(),
                    };

                    ws_stream.lock().await.send(Message::from(serde_json::to_string(&res).expect("serialize rooms"))).await.expect("Fails to send initils info");
                }
                
                Ok(IncomingMessage::AcessRoom(room)) => {
                    println!("User {:?} required acess to {:?}", ref_user, room.code);
                    
                    let mut acess_room = rooms.lock().await.get(&room.code).unwrap().clone();
                    if acess_room.info.password.eq(&room.password) {
                        acess_room.users.push(ref_user.clone());
                        let res = ServerResponse{
                            for_action: Action::AcessRoom,
                            res_type: ResType::Success,
                            message: "Accessed room successfully".to_string(),
                        };
                        
                        ws_stream.lock().await.send(Message::from(serde_json::to_string(&res).expect("serialize res"))).await.expect("Fails to send server res");
                    }else {
                        let res = ServerResponse{
                            for_action: Action::AcessRoom,
                            res_type: ResType::Error,
                            message: "Incorrect Password".to_string(),
                        };

                        ws_stream.lock().await.send(Message::from(serde_json::to_string(&res).expect("serialize res"))).await.expect("Fails to send server res");
                    }
                    
                }
                
                
                Ok(IncomingMessage::UserMessage(user_message)) => {
                    let mut new_rooms = HashMap::new();
                    new_rooms.insert(user_message.room.clone(), rooms.lock().await.get(&user_message.room).unwrap().clone());
                    let rooms = Arc::new(Mutex::new(new_rooms));
                    broadcast(users.clone(), rooms, Message::from(serde_json::to_string(&user_message.message).expect("serialize rooms"))).await;

                }

                Err(e) => {
                    let res = ServerResponse {
                        for_action: Action::Connect,
                        res_type: ResType::Error,
                        message: e.to_string(),
                    };

                    ws_stream.lock().await.send(Message::from(serde_json::to_string(&res).expect("serialize rooms"))).await.expect("Sendo error message");
                },
            }
        }
    }

    Ok(())
}


async fn broadcast(
    users: Arc<Mutex<HashMap<Uuid, (User, Arc<Mutex<WebSocketStream<TcpStream>>>)>>>,
    rooms: Arc<Mutex<HashMap<String, Room>>>,
    message: Message,
) {
    for room in rooms.lock().await.values() {
        let room_clone = room.clone();
        let users_in_room = room_clone.users;
        
        let users_locked = users.lock().await;
        let room_users = users_locked.iter().filter(|(_, (user, _))| {
            users_in_room.contains(user)
        });

        for (_, (_, ws)) in room_users {
            ws.lock().await.send(message.clone()).await.expect("send broadcast message");
        }
    }
}

