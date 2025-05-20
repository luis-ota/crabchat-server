use crate::infra::models::{AcessRoom, CreateRoom, User, UserMessage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum IncomingMessage {
    User(User),
    CreateRoom(CreateRoom),
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
pub enum Action {
    Connect,
    CreateRoom,
    AcessRoom,
}
