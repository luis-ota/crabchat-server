use std::{str::FromStr, sync::Arc};

use futures_util::{SinkExt, stream::SplitSink};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::WebSocketStream;
use tracing::{info, instrument};
use tungstenite::Message;
use uuid::Uuid;

use crate::{
    auth::{check_session, register_user},
    infra::{
        enums::{Action, IncomingMessage, ResType, ServerError},
        models::{AvaliableRoom, ServerMessage, ToJson, User},
    },
    room::{acess_room, create_room, delete_room, get_room_users, leave_room},
    types::{SharedRooms, SharedUsers},
};

#[instrument(skip(users, rooms, ws_sender))]
pub async fn process_message(
    msg: IncomingMessage,
    user_id: &Uuid,
    current_user: &mut Option<User>,
    users: &SharedUsers,
    rooms: &SharedRooms,
    ws_sender: &Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
) -> Result<(), ServerError> {
    match msg {
        IncomingMessage::User(user_data) => {
            register_user(current_user, user_data, user_id, users, ws_sender).await?;
            send_rooms(rooms, ws_sender).await?;
        }
        _ => {
            let user = check_session(current_user).await?;

            match msg {
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
                        get_room_users(rooms, &user_message.room_code).await?,
                        user_message.to_json()?,
                        Some(vec![user]),
                    )
                    .await?
                }
                IncomingMessage::User(_) => unreachable!("this case is already handled"),
            }
        }
    }
    Ok(())
}

pub async fn server_response(
    ws_sender: &Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
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

pub async fn send_rooms(
    rooms: &SharedRooms,
    ws_sender: &Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
) -> Result<(), ServerError> {
    let avaliable_rooms: Vec<AvaliableRoom> = rooms
        .lock()
        .await
        .values()
        .filter(|r| r.info.public)
        .map(|r| AvaliableRoom {
            info: r.public_info(),
            users_count: r.users.len() as u64,
            has_password: r.info.password.is_some(),
        })
        .collect();

    let avaliable_rooms_json =
        serde_json::to_string(&avaliable_rooms).map_err(ServerError::Serialization)?;
    info!(
        "trying to send avaliable rroms: {:#?}",
        avaliable_rooms_json
    );
    ws_sender
        .lock()
        .await
        .send(Message::from(avaliable_rooms_json))
        .await
        .map_err(ServerError::WebSocket)?;

    info!("the avalible rooms ware succefully sent");
    Ok(())
}

pub async fn broadcast(
    users_ws_streams: &SharedUsers,
    users_to_bc: Vec<User>,
    message: String,
    skip: Option<Vec<&User>>,
) -> Result<(), ServerError> {
    let users_sw_lock = users_ws_streams.lock().await;

    for user in users_to_bc {
        let uuid = Uuid::from_str(&user.uuid)
            .map_err(|_| ServerError::InvalidUuid(user.uuid.to_owned()))?;

        if let Some(ref skips) = skip {
            if skips.iter().any(|u| u.uuid == user.uuid) {
                continue;
            }
        }

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
