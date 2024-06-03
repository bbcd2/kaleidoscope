//! Callbacks for any given connection

use crate::{
    connection::messages::ServerMessage,
    database::{Database, Video},
    ClientConnections,
};

use std::{
    fmt::{Display, Result as FmtResult},
    pin::Pin,
    sync::Arc,
};

use anyhow::{anyhow, Context as _, Result};
use futures_util::Future;
use tokio::sync::Mutex;
use warp::filters::ws::Message;

pub enum CallbackError {
    /// An error that should not be returned the the client
    Silent(anyhow::Error),
    /// An error that should be returned to the client
    Loud(anyhow::Error),
}
/// Default errors are loud
impl From<anyhow::Error> for CallbackError {
    fn from(value: anyhow::Error) -> Self {
        Self::Loud(value)
    }
}
impl Display for CallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> FmtResult {
        match self {
            Self::Silent(e) => write!(f, "{}", e),
            Self::Loud(e) => write!(f, "{}", e),
        }
    }
}

/// State passed into all message handlers and callbacks
#[derive(Clone)]
pub struct MessageHandlerState {
    pub client_id: usize,
    pub clients: ClientConnections,
    // Although [`Database`] implements Clone, it doesn't implement Send + Sync
    pub database: Arc<Mutex<Database>>,
}

/// A response from a message handler or callback that determines what to return to the client
pub type MessageHandlerResponse = Result<Option<ServerMessage>, CallbackError>;
pub type AsynchronousMessageHandlerResponse =
    Pin<Box<dyn Future<Output = MessageHandlerResponse> + Send>>;

pub fn on_connect(_state: Arc<MessageHandlerState>) -> AsynchronousMessageHandlerResponse {
    Box::pin(async move { Ok(None) })
}
pub fn on_disconnect(_state: Arc<MessageHandlerState>) -> AsynchronousMessageHandlerResponse {
    Box::pin(async move { Ok(None) })
}
pub fn on_message(
    _state: Arc<MessageHandlerState>,
    _message: Message,
) -> AsynchronousMessageHandlerResponse {
    Box::pin(async move { Ok(None) })
}

pub async fn alert_clients_of_database_change(
    clients: ClientConnections,
    change: &Video,
) -> Result<()> {
    let clients = clients.read().await;
    for (id, client) in clients.iter() {
        client
            .send(Message::text(serde_json::to_string(
                &ServerMessage::DatabaseUpdate(change.clone()),
            )?))
            .with_context(|| anyhow!("sending to client {id}"))?;
    }
    Ok(())
}