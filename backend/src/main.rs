//! The backend for BBCD!!!

pub mod callbacks;
pub mod connection;

use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};

use callbacks::{on_connect, on_disconnect, on_message};
use connection::handle_connection;
use log::info;
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use warp::{ws::Message, Filter as _};

pub const PORT: u16 = 8081;

pub type ClientConnections = Arc<RwLock<HashMap<usize, UnboundedSender<Message>>>>;

pub static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(0);

#[tokio::main]
async fn main() {
    std::env::set_var(
        "RUST_LOG",
        std::env::var("RUST_LOG")
            .and_then(|arg| match arg.len() {
                0 => Err(std::env::VarError::NotPresent),
                _ => Ok(arg),
            })
            .unwrap_or("debug".to_string()),
    );
    pretty_env_logger::init();
    info!("hello from the bbcd backend!");
    let log = warp::log("bbcd-backend");
    let cors = warp::cors();

    /* State */
    let clients_filter = {
        let clients = ClientConnections::default();
        warp::any().map(move || clients.clone())
    };

    /* Root route */
    let root = warp::path!()
        .map(|| "hey, welcome to the bbcd backend! uh, please leave /lh".to_owned())
        .with(cors)
        .with(log);

    /* Websocket route */
    let websocket_route = warp::path("websocket")
        .and(warp::path::end())
        .and(warp::ws())
        /* State */
        .and(clients_filter)
        .map(|ws: warp::ws::Ws, clients: ClientConnections| {
            ws.on_upgrade(move |socket| {
                handle_connection(
                    socket,
                    /* Callbacks */
                    on_connect,
                    on_disconnect,
                    on_message,
                    /* State */
                    clients,
                )
            })
        });

    let routes = warp::get().and(root.or(websocket_route));

    info!("running on port {PORT}!");
    warp::serve(routes).run(([0; 4], PORT)).await;
}
