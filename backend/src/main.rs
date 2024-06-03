//! The backend for BBCD!!!

pub mod callbacks;
pub mod connection;
pub mod database;
pub mod filters;
pub mod schema;

use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};

use anyhow::Result;
use dotenvy::dotenv;
use log::info;
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use warp::{ws::Message, Filter as _};

use crate::filters::{root_route, websocket_route};

pub const PORT: u16 = 8081;

pub type ClientConnections = Arc<RwLock<HashMap<usize, UnboundedSender<Message>>>>;

pub static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(0);

fn init_log() {
    std::env::set_var(
        "RUST_LOG",
        std::env::var("RUST_LOG")
            .and_then(|arg| match arg.len() {
                0 => Err(std::env::VarError::NotPresent),
                _ => Ok(arg),
            })
            .unwrap_or("info".to_string()),
    );
    pretty_env_logger::init();
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log();
    info!("hello from the bbcd backend!");

    let pool = database::establish_connection()?;

    let routes = warp::get().and(root_route().or(websocket_route(pool)));

    info!("running on port {PORT}!");
    warp::serve(routes).run(([0; 4], PORT)).await;

    Ok(())
}
