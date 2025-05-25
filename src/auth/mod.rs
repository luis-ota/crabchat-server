use std::sync::Arc;

use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::WebSocketStream;
use tracing::info;
use uuid::Uuid;

use crate::{
    infra::{enums::ServerError, models::User},
    types::SharedUsers,
};

pub async fn register_user(
    current_user: &mut Option<User>,

    user_data: User,
    user_id: &Uuid,
    users: &SharedUsers,
    ws_sender: &Arc<Mutex<WebSocketStream<TcpStream>>>,
) -> Result<(), ServerError> {
    info!("Usuário se identificou: {:?}", user_data);

    let new_user = User {
        uuid: user_id.to_string(),
        ..user_data
    };

    users
        .lock()
        .await
        .insert(user_id.to_owned(), Arc::clone(ws_sender));

    *current_user = Some(new_user);
    Ok(())
}

pub async fn check_session(current_user: &mut Option<User>) -> Result<&User, ServerError> {
    current_user.as_ref().ok_or(ServerError::Unauthorized)?;
    Ok(current_user.as_ref().unwrap())
}
