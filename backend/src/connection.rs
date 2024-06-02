//! Facilitates a client's connection

use crate::{
    callbacks::{
        AsynchronousMessageHandlerResponse, CallbackError, MessageHandlerResponse,
        MessageHandlerState,
    },
    connection::messages::ServerMessage,
    ClientConnections, NEXT_CLIENT_ID,
};

use std::sync::{atomic::Ordering::Relaxed, Arc};

use futures_util::{SinkExt as _, StreamExt as _, TryFutureExt as _};
use log::error;
use tokio::sync::mpsc::unbounded_channel;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

/// Messages
pub mod messages {
    #[derive(serde::Deserialize, serde::Serialize)]
    pub enum ClientMessage {}
    #[derive(serde::Deserialize, serde::Serialize)]
    pub enum ServerMessage {
        Error(String),
    }
}

/// Bridge for a client's connection that listens for messages and allows for them
/// to be sent with a Tx in the `ClientConnections` object.
/// Calls `on_message_callback` with the client ID and the message every time the
/// client sends a message.
/// Calls `on_connect_callback` with the client ID once the client is connected and
/// in the `ClientConnections` object.
/// Calls `on_disconnect_callback` with the client ID once the client is disconnected
/// but before they are removed from the `ClientConnections` object.
pub async fn handle_connection<C, D, M>(
    ws: WebSocket,
    /* Callbacks */
    on_connect_callback: C,
    on_disconnect_callback: D,
    on_message_callback: M,
    /* State */
    clients: ClientConnections,
) where
    C: Fn(Arc<MessageHandlerState>) -> AsynchronousMessageHandlerResponse,
    D: Fn(Arc<MessageHandlerState>) -> AsynchronousMessageHandlerResponse,
    M: Fn(Arc<MessageHandlerState>, Message) -> AsynchronousMessageHandlerResponse,
{
    let client_id = NEXT_CLIENT_ID.fetch_add(1, Relaxed);

    // Create channels
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = unbounded_channel::<Message>();
    let (private_tx, mut rx) = (tx.clone(), UnboundedReceiverStream::new(rx));

    // Helper for a callback with error handling and sending through our Tx
    let callback_state = Arc::new(MessageHandlerState { client_id });
    let callback_handle = |response: MessageHandlerResponse, callback: &'static str| match response
    {
        Ok(Some(send_back)) => {
            let _ = private_tx.send(Message::text(serde_json::to_string(&send_back).unwrap()));
        }
        Err(CallbackError::Loud(e)) => {
            error!("client with ID {client_id}: sending error in {callback} callback: {e}");
            let _ = private_tx.send(Message::text(
                serde_json::to_string(&ServerMessage::Error(e.to_string())).unwrap(),
            ));
        }
        Err(CallbackError::Silent(e)) => {
            error!("client with ID {client_id}: ignoring error in {callback} callback: {e}")
        }
        _ => (),
    };

    // Connection
    clients.write().await.insert(client_id, tx);
    callback_handle(
        on_connect_callback(Arc::clone(&callback_state)).await,
        "connection",
    );

    // Backend Rx -> websocket Tx bridge
    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            ws_tx
                .send(message)
                .unwrap_or_else(|e| {
                    error!("client with ID {client_id}: ignoring websocket error: {e}");
                })
                .await;
        }
    });

    // Websocket Rx -> backend Tx bridge
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(message) => callback_handle(
                on_message_callback(Arc::clone(&callback_state), message).await,
                "message",
            ),
            Err(_) => break,
        }
    }

    // No more messages, client must have disconnected
    // (since the backend Tx is going nowhere, `callback_handle` is overkill for this)
    callback_handle(
        on_disconnect_callback(Arc::clone(&callback_state)).await,
        "disconnection",
    );
    clients.write().await.remove(&client_id);
}
