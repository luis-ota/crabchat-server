use crate::infra::models::{AcessRoom, CreateRoom, DeleteRoom, User, UserMessage};
use serde::{Deserialize, Serialize};

use super::models::ServerResponse;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum IncomingMessage {
    User(User),
    CreateRoom(CreateRoom),
    DeleteRoom(DeleteRoom),
    AcessRoom(AcessRoom),
    UserMessage(UserMessage),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ResType {
    Success,
    Error,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum BroadCastMessage {
    UserMessage(UserMessage),
    ServerMessage(ServerResponse),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Action {
    Connect,
    CreateRoom,
    DeleteRoom,
    AcessRoom,
}
