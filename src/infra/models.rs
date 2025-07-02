use crate::infra::enums::{Action, ResType};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::enums::ServerError;

pub trait ToJson {
    fn to_json(&self) -> Result<String, ServerError>
    where
        Self: Serialize,
    {
        serde_json::to_string(self).map_err(ServerError::Serialization)
    }
}

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
    pub room_code: String,
}

impl UserMessage {
    pub fn new(user: &User, message: &str, room_code: &String) -> Result<Self, ServerError> {
        Ok(Self {
            user: Some(user.to_owned()),
            message: message.to_string(),
            datetime: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            room_code: room_code.to_string(),
        })
    }
}

impl ToJson for UserMessage {}

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
impl ToJson for CreateRoom {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRoom {
    pub code: String,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRoom {
    pub room_code: String,
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
pub struct AvailableRoom {
    pub info: CreateRoom,
    pub users_count: u64,
    pub has_password: bool,
}

impl Room {
    pub fn public_info(&self) -> CreateRoom {
        CreateRoom {
            base_info: self.info.base_info.clone(),
            password: None,
            public: self.info.public,
        }
    }
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

impl ToJson for ServerMessage {}
