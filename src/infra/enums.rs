use serde::{Serialize, Deserialize};
use crate::infra::models::{AcessRoom, CreateRoom, User, UserMessage};

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
