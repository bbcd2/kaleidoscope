//! Tree clearing for our logging service

use std::io::Write as _;

use chrono::Local;
use env_logger::fmt::{style::Style, Formatter};
use log::{info, Level, Record};
use warp::filters::log::Info;

fn format_write(buffer: &mut Formatter, record: &Record) -> std::io::Result<()> {
    let level = record.level();
    let mut bold = Style::new();
    if matches!(level, Level::Info) {
        bold = bold.bold();
    };
    let level_color = buffer.default_level_style(level);
    for result in record.args().to_string().split('\n').map(|line| {
        write!(
            buffer,
            "[{time} {bold}{level_color}{level:<5}{level_color:#}{bold:#}] {bold}{line}{bold:#}\n",
            time = Local::now().format("%Y-%m-%d %H:%M:%S"),
        )
    }) {
        result?
    }
    Ok(())
}

pub fn init_logger() {
    env_logger::builder()
        .parse_env(
            env_logger::Env::default()
                .filter_or(env_logger::DEFAULT_FILTER_ENV, "info,backend=debug"), // .default_write_style_or("always"),
        )
        .format(format_write)
        .init();
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
