use std::{collections::HashMap, sync::Arc};

use futures_util::StreamExt;
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::accept_async;
use tracing::{error, info, instrument, warn};
use tungstenite::Message;
use uuid::Uuid;

use crate::{
    infra::{
        enums::{Action, IncomingMessage, ResType, ServerError},
        models::{Room, User},
    },
    message::{process_message, server_response},
    types::SharedUsers,
};

#[instrument(skip_all)]
pub async fn handle_connection(
    stream: TcpStream,
    users: SharedUsers,
    rooms: Arc<Mutex<HashMap<String, Room>>>,
) -> Result<(), ServerError> {
    let ws_stream = Arc::new(Mutex::new(accept_async(stream).await?));
    info!("conexão websocket estabelecida com sucesso.");

    let user_id = Uuid::new_v4();
    let mut current_user: Option<User> = None;

    while let Some(msg) = ws_stream.lock().await.next().await {
        if let Ok(Message::Text(text)) = msg {
            match serde_json::from_str::<IncomingMessage>(&text) {
                Ok(incoming_message) => {
                    let result = process_message(
                        incoming_message,
                        &user_id,
                        &mut current_user,
                        &users,
                        &rooms,
                        &Arc::clone(&ws_stream),
                    )
                    .await;

                    match result {
                        Ok(_) => {
                            server_response(
                                &ws_stream,
                                Action::Request,
                                ResType::Success,
                                "".to_string(),
                            )
                            .await?
                        }
                        Err(e) => {
                            warn!("erro ao processar a mensagem: {}. texto: {}", e, text);
                            server_response(
                                &ws_stream,
                                Action::Request,
                                ResType::InvalidRequest,
                                format!("erro ao processar a mensagem: {}. texto: {}", e, text),
                            )
                            .await?
                        }
                    }
                }
                Err(e) => {
                    error!("erro ao desserializar a mensagem: {}. texto: {}", e, text);
                }
            }
        }
    }

    Ok(())
}
