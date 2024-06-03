//! Tree clearing for our logging service

use log::info;
use warp::filters::log::Info;

pub fn init_logger() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
}

/// A filter to log a request. Ignores any 4xx response codes
pub fn get_warp_logger(info: Info) {
    if info.status().as_u16() >= 400 && info.status().as_u16() < 500 {
        return;
    }
    info!(
        "{}: {} to {} returned {} and took {}ms",
        info.remote_addr()
            .map_or_else(|| "unknown".to_string(), |address| address.to_string()),
        info.method(),
        info.path(),
        info.status(),
        info.elapsed().as_millis()
    );
}
