//! The backend for BBCD!!!

pub mod callbacks;
pub mod connection;
pub mod database;
pub mod filters;
pub mod schema;
pub mod tree;

use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};

use anyhow::Result;
use dotenvy::dotenv;
use log::info;
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use warp::{ws::Message, Filter as _};

use filters::{list_recordings, root_route, websocket_route};
use tree::init_logger;

pub const PORT: u16 = 8081;

pub type ClientConnections = Arc<RwLock<HashMap<usize, UnboundedSender<Message>>>>;

pub static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(0);

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_logger();
    info!("hello from the bbcd backend!");

    let pool = database::establish_connection()?;

    let routes = warp::get().and(
        root_route()
            .or(list_recordings(pool.clone()))
            .or(websocket_route(pool)),
    );

    info!("running on port {PORT}!");
    warp::serve(routes).run(([0; 4], PORT)).await;

    Ok(())
}
