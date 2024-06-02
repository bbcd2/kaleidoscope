//! Callbacks for any given connection

use crate::connection::messages::{ClientMessage, ServerMessage};

use std::{
    fmt::{Display, Result as FmtResult},
    pin::Pin,
    sync::Arc,
};

use anyhow::{anyhow, Result};
use futures_util::Future;
use log::{debug, info};
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
}

/// A response from a message handler or callback that determines what to return to the client
pub type MessageHandlerResponse = Result<Option<ServerMessage>, CallbackError>;
pub type AsynchronousMessageHandlerResponse =
    Pin<Box<dyn Future<Output = MessageHandlerResponse> + Send>>;

/// Initialize handles to the client. Note that the client is not in a room until they send
/// a join message.
pub fn on_connect(state: Arc<MessageHandlerState>) -> AsynchronousMessageHandlerResponse {
    Box::pin(async move { Ok(None) })
}

/// Clean up all handles to the client
pub fn on_disconnect(state: Arc<MessageHandlerState>) -> AsynchronousMessageHandlerResponse {
    Box::pin(async move {
        info!("client with ID {id} disconnected", id = state.client_id);
        Ok(None)
    })
}

/// Call the message handlers for a given message from the client
pub fn on_message(
    state: Arc<MessageHandlerState>,
    message: Message,
) -> AsynchronousMessageHandlerResponse {
    Box::pin(async move {
        debug!(
            "client with ID {id} sent [unparsed] message `{message:?}`",
            id = state.client_id,
            message = message.to_str()
        );
        let message = message
            .to_str()
            .map_err(|_| CallbackError::Loud(anyhow!("websocket message was not a `str`")))?;
        info!(
            "client with ID {id} sent message `{message}`",
            id = state.client_id,
            message = message.trim(),
        );
        // Parse
        let _message = serde_json::from_str::<ClientMessage>(message)
            .map_err(|e| CallbackError::Silent(anyhow!("could not parse message: {e}")))?;

        Ok(None)
    })
}
