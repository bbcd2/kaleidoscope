//! The backend for BBCD!!!

pub mod clip;
pub mod consts;
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

use std::thread;

use anyhow::Result;
use clip::FfmpegProgressChannels;
use dotenvy::dotenv;
use filters::ffmpeg_progress;
use log::{debug, info, trace};
use tokio::runtime::Runtime;
use warp::Filter as _;
use websocket_connection::ClientConnections;

pub const PORT: u16 = 8081;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_logger();
    debug!("hello from the bbcd backend!");
    trace!("trace enabled!");

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
        let ffmpeg_progress_channels = FfmpegProgressChannels::default();

        let routes = root_route()
            .or(websocket_route(pool.clone(), clients.clone()))
            .or(list_recordings(pool.clone()))
            .or(clip_route(
                pool.clone(),
                clip_runtime,
                clients.clone(),
                ffmpeg_progress_channels.clone(),
            ))
            .or(ffmpeg_progress(ffmpeg_progress_channels.clone()));

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
