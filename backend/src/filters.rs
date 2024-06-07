use crate::{
    clip::{clip, ffmpeg_progress_update_handler, ClipParameters, FfmpegProgressChannels},
    database::{with_database, Database, PoolPg},
    tree::get_warp_logger,
    websocket_callbacks::{on_connect, on_disconnect, on_message},
    websocket_connection::handle_connection,
    ClientConnections,
};

use std::{collections::HashMap, fmt::Debug, net::SocketAddr};

use anyhow::anyhow;
use serde::de::DeserializeOwned;
use tokio::runtime::Handle;
use uuid::Uuid;
use warp::{reject::Reject, Filter};

/// Wrapper for a Warp rejection message
pub struct ServerError {
    message: String,
}
impl ServerError {
    pub fn new<M>(message: M) -> Self
    where
        M: ToString,
    {
        Self {
            message: message.to_string(),
        }
    }
}
impl Debug for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{message}", message = self.message)
    }
}
impl Reject for ServerError {}

/// Filter for accepting a thread-safe value in a handler
pub fn with<T>(something: T) -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
where
    T: Clone + Sync + Send,
{
    warp::any()
        .map(move || something.clone())
        .and_then(|something: T| async move { Ok::<_, warp::Rejection>(something) })
}

/// Filter for accepting a JSON body
fn with_json_body<T: DeserializeOwned + Send>(
) -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone {
    const MAX_SIZE: u64 = 4 * 1024 /* KiB */;
    warp::body::content_length_limit(MAX_SIZE).and(warp::body::json())
}

/// GET /
pub fn root_route() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path!())
        .and(warp::path::end())
        .map(|| "hey, welcome to the bbcd backend! uh, please leave /lh".to_owned())
        .with(warp::cors())
        .with(warp::log::custom(get_warp_logger))
}

/// GET /list-recordings
pub fn list_recordings(
    pool: PoolPg,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path!("list-recordings"))
        .and(warp::path::end())
        .and(warp::query::<HashMap<String, String>>())
        .and(with_database(pool))
        .and_then(
            |query: HashMap<String, String>, mut database: Database| async move {
                let (start, count) = (query.get("start"), query.get("count"));
                let start = start.map(|start| start.parse().ok()).flatten().unwrap_or(0);
                let count = count
                    .map(|count| count.parse().ok())
                    .flatten()
                    .unwrap_or(15);
                match database.get_recordings(start, count) {
                    Ok(videos) => Ok(warp::reply::json(&videos)),
                    Err(e) => Err(warp::reject::custom(ServerError::new(anyhow!(
                        "failed to fetch videos: {e}"
                    )))),
                }
            },
        )
        .with(warp::cors())
        .with(warp::log::custom(get_warp_logger))
}

/// POST /clip
pub fn clip_route(
    pool: PoolPg,
    clip_runtime: Handle,
    clients: ClientConnections,
    ffmpeg_progress_channels: FfmpegProgressChannels,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path!("clip"))
        .and(warp::path::end())
        .and(with_json_body::<ClipParameters>())
        .and(with(clients))
        .and(with(ffmpeg_progress_channels))
        .and(with_database(pool))
        .map(
            move |parameters: ClipParameters,
                  clients: ClientConnections,
                  ffmpeg_progress_channels: FfmpegProgressChannels,
                  database: Database| {
                let uuid = Uuid::new_v4().to_string();
                clip_runtime.spawn(clip(
                    uuid.clone(),
                    parameters.channel,
                    [parameters.start_timestamp, parameters.end_timestamp],
                    parameters.encode,
                    database,
                    clients,
                    ffmpeg_progress_channels,
                ));
                uuid
            },
        )
        .with(warp::log::custom(get_warp_logger))
}

/// POST /ffmpeg-progress
pub fn ffmpeg_progress(
    ffmpeg_progress_channels: FfmpegProgressChannels,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("ffmpeg-progress"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(with(ffmpeg_progress_channels))
        .and(warp::filters::addr::remote())
        .and(warp::body::stream())
        .and_then(
            |uuid: String,
             ffmpeg_progress_channels: FfmpegProgressChannels,
             remote: Option<SocketAddr>,
             body| async move {
                if !remote.ok_or(warp::reject::not_found())?.ip().is_loopback() {
                    Err(warp::reject::not_found())?;
                }
                ffmpeg_progress_update_handler(body, uuid, ffmpeg_progress_channels)
                    .await
                    .map_err(|e| warp::reject::custom(ServerError::new(e)))
            },
        )
        .with(warp::log::custom(get_warp_logger))
}

/// GET /websocket
pub fn websocket_route(
    pool: PoolPg,
    clients: ClientConnections,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path!("websocket"))
        .and(warp::path::end())
        .and(warp::ws())
        /* State */
        .and(with(clients))
        .and(with_database(pool))
        .map(
            |ws: warp::ws::Ws, clients: ClientConnections, database: Database| {
                ws.on_upgrade(move |socket| {
                    handle_connection(
                        socket,
                        /* Callbacks */
                        on_connect,
                        on_disconnect,
                        on_message,
                        /* State */
                        clients,
                        database,
                    )
                })
            },
        )
}
