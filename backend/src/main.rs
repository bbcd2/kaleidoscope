//! The backend for BBCD!!!

pub mod clip;
pub mod database;
pub mod filters;
pub mod schema;
pub mod tree;
pub mod websocket_callbacks;
pub mod websocket_connection;

use crate::{
    filters::{clip_route, list_recordings, root_route, websocket_route},
    tree::init_logger,
};

use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
    thread,
};

use anyhow::Result;
use dotenvy::dotenv;
use log::info;
use tokio::{
    runtime::Runtime,
    sync::{mpsc::UnboundedSender, RwLock},
};
use warp::{ws::Message, Filter as _};

pub const PORT: u16 = 8081;

pub type ClientConnections = Arc<RwLock<HashMap<usize, UnboundedSender<Message>>>>;

pub static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_logger();
    info!("hello from the bbcd backend!");

    let (clip_shutdown_tx, clip_shutdown_rx) = tokio::sync::oneshot::channel();
    let (clip_handle_tx, clip_handle_rx) = std::sync::mpsc::channel();

    let clip_thread = thread::spawn(move || {
        let runtime = Runtime::new().expect("failed to create clip runtime");

        clip_handle_tx
            .send(runtime.handle().clone())
            .expect("failed to share clip runtime with webserver thread");

        runtime.block_on(async move {
            clip_shutdown_rx
                .await
                .expect("clip runtime failed to listen on shutdown channel");
        })
    });

    let webserver_thread = thread::spawn(move || {
        let runtime = Runtime::new().expect("failed to create webserver runtime");

        let clip_runtime = clip_handle_rx
            .recv()
            .expect("failed to get the clip runtime");

        let pool =
            database::establish_connection().expect("failed to establish database connection");

        let clients = ClientConnections::default();

        let routes = root_route()
            .or(websocket_route(pool.clone(), clients.clone()))
            .or(list_recordings(pool.clone()))
            .or(clip_route(pool.clone(), clients.clone(), clip_runtime));

        runtime.block_on(async move {
            info!("running on port {PORT}!");
            warp::serve(routes).run(([0; 4], PORT)).await;
        });
    });

    webserver_thread.join().expect("webserver thread panicked");
    clip_shutdown_tx
        .send(())
        .expect("failed to send clip runtime shutdown signal");
    clip_thread.join().expect("clip thread panicked");

    Ok(())
}
