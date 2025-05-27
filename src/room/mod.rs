use std::{collections::HashMap, sync::Arc};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::WebSocketStream;
use tracing::info;

use crate::{
    infra::{
        enums::{Action, ResType, ServerError},
        models::{AccessRoom, CreateRoom, DeleteRoom, LeaveRoom, Room, ToJson, User, UserMessage},
    },
    message::{broadcast, server_response},
    types::{SharedRooms, SharedUsers},
};

pub async fn create_room(
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

pub async fn delete_room(
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
                None,
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

pub async fn get_room_users(
    rooms: &SharedRooms,
    room_code: &String,
) -> Result<Vec<User>, ServerError> {
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

pub async fn acess_room(
    user: &User,
    acess_room: AccessRoom,
    rooms: &SharedRooms,
    users: &SharedUsers,
    ws_sender: &Arc<Mutex<WebSocketStream<TcpStream>>>,
) -> Result<(), ServerError> {
    info!(
        "User {:?} required acess to room {:?}",
        user, acess_room.room_code
    );

    let mut room = get_room(rooms, &acess_room.room_code).await?;

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

        let user_message = UserMessage::new(user, "joined room", &acess_room.room_code)?;

        broadcast(
            users,
            get_room_users(rooms, &acess_room.room_code).await?,
            user_message.to_json()?,
            None,
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

pub async fn leave_room(
    user: &User,
    rooms: &SharedRooms,
    users: &SharedUsers,
    leave_room: &LeaveRoom,
) -> Result<(), ServerError> {
    let mut room = get_room(rooms, &leave_room.code).await?;

    room.users.remove(&user.uuid);
    let message = UserMessage::new(user, "leaved room", &leave_room.code)?;

    broadcast(
        users,
        get_room_users(rooms, &leave_room.code).await?,
        message.to_json()?,
        None,
    )
    .await?;

    Ok(())
}
