use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;
use crate::infra::enums::{Action, ResType};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(PartialEq)]
pub struct User {
    pub name: String,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub message: String,
    pub datetime: String,
    pub room: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseRoomInfo {
    pub code: String,
    pub name: String,
    pub created_by: User,

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoom {
    pub base_info: BaseRoomInfo,
    pub password: Option<String>,
    pub public: bool,

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcessRoom {
    pub code: String,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub info: CreateRoom,
    pub messages: Vec<UserMessage>,
    pub users: Vec<User>,

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoomInfo {
    pub base_info: BaseRoomInfo,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerResponse {
    pub for_action: Action,
    pub res_type: ResType,
    pub message: String,
}