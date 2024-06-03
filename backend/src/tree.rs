use log::info;
use warp::filters::log::Info;

pub fn init_logger() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
}

pub fn get_warp_logger(info: Info) {
    if info.status() == warp::http::StatusCode::NOT_FOUND {
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
