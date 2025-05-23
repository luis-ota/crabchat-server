use std::collections::HashMap;

use crate::infra::enums::{Action, ResType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub name: String,
    pub uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub user: Option<User>,
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
pub struct DeleteRoom {
    pub room: String,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcessRoom {
    pub code: String,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveRoom {
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub info: CreateRoom,
    pub messages: Vec<UserMessage>,
    pub users: HashMap<String, User>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoomInfo {
    pub base_info: BaseRoomInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMessage {
    pub for_action: Action,
    pub res_type: ResType,
    pub message: String,
}
