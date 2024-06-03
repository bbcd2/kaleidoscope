use std::fmt::Debug;

use anyhow::anyhow;
use warp::{
    reject::{self, Reject},
    Filter,
};

use crate::{
    callbacks::{on_connect, on_disconnect, on_message},
    connection::handle_connection,
    database::{Database, PoolPg},
    ClientConnections,
};

struct ServerError {
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

fn with_db_access_manager(
    pool: PoolPg,
) -> impl Filter<Extract = (Database,), Error = warp::Rejection> + Clone {
    warp::any()
        .map(move || pool.clone())
        .and_then(|pool: PoolPg| async move {
            match pool.get() {
                Ok(pool) => Ok(Database { connection: pool }),
                Err(e) => Err(reject::custom(ServerError::new(anyhow!(
                    "failed to access database: {e}"
                )))),
            }
        })
}

pub fn root_route() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!()
        .map(|| "hey, welcome to the bbcd backend! uh, please leave /lh".to_owned())
        .with(warp::cors())
        .with(warp::log("root"))
}

pub fn websocket_route(
    pool: PoolPg,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let clients_filter = {
        let clients = ClientConnections::default();
        warp::any().map(move || clients.clone())
    };
    warp::path!("websocket")
        .and(warp::path::end())
        .and(warp::ws())
        /* State */
        .and(clients_filter)
        .and(with_db_access_manager(pool))
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
