use crate::{
    database::{Database, RecordingUpdate},
    websocket_callbacks::alert_clients_of_database_change,
    ClientConnections,
};

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::DateTime;
use log::{error, info};
use serde::Deserialize;
use tokio::fs::{create_dir_all, remove_dir_all};

/// Where to store segment and job files
const TEMP_DIRECTORY: &str = "temp";

/// Parameters from the request
#[derive(Deserialize)]
pub struct ClipParameters {
    pub start_timestamp: usize,
    pub end_timestamp: usize,
    pub channel: usize,
    pub encode: bool,
}

/// Clip progress stage
#[derive(Clone, Copy, Debug)]
#[repr(usize)]
pub enum Stage {
    // OK statuses
    Initializing = 1,
    Downloading = 2,
    Combining = 3,
    Encoding = 4,
    Uploading = 5,
    Complete = 6,
    /// Separation between OK statuses and error statuses
    #[allow(non_camel_case_types)]
    _SENTINEL_MAX_OK = 7,
    // Error statuses
    FailedNondescript = 10,
    DownloadingFailed = 11,
    CombiningFailed = 12,
    EncodingFailed = 13,
    UploadingFailed = 14,
}
impl Stage {
    pub fn error_variant(self) -> Self {
        match self {
            Self::Downloading => Self::DownloadingFailed,
            Self::Combining => Self::CombiningFailed,
            Self::Encoding => Self::EncodingFailed,
            Self::Uploading => Self::UploadingFailed,
            _ => Self::FailedNondescript,
        }
    }
}

/// Convert the timestamp to a segment index (referred to in the digest as $Number$)
fn calculate_segment_idx(timestamp: usize) -> usize {
    // <SegmentTemplate ... timescale="50" duration="192" />
    const MAGIC_OFFSET: usize = 38; // 145.92 seconds
    (((timestamp as f64) / (192. / 50.)).floor() as usize) + MAGIC_OFFSET
}

/// Download segments to the resultant directory
async fn download_segments(
    uuid: &str,
    channel: usize,
    segment_idx_bounds: [usize; 2],
    target_directory: &Path,
) -> Result<()> {
    Err(anyhow!("not implemented sorry :3"))?;
    Ok(())
}

pub async fn clip(
    clients: ClientConnections,
    uuid: String,
    channel: usize,
    timeframe: [usize; 2],
    encode: bool,
    mut database: Database,
) -> Result<()> {
    info!("{uuid}: starting clip");

    let mut stage = Stage::Initializing;

    let mut timestamp_bounds = timeframe.iter().map(|bound| -> Result<_> {
        Ok(DateTime::from_timestamp(*bound as i64, 0)
            .context("failed to convert bounds to timestamp")?
            .naive_utc())
    });

    let mut recording_row = RecordingUpdate {
        user_id: None,
        uuid: &uuid,
        rec_start: &timestamp_bounds.next().unwrap()?,
        rec_end: &timestamp_bounds.next().unwrap()?,
        stage: stage as usize as i32,
        status: format!("{stage:?}"),
        channel: channel as i32,
    };
    database.create_recording(&recording_row)?;

    let output_directory = PathBuf::from(TEMP_DIRECTORY).join(&uuid);

    let closure = async {
        let segment_idx_bounds = timeframe.map(|bound| calculate_segment_idx(bound));

        create_dir_all(&output_directory).await?;
        download_segments(&uuid, channel, segment_idx_bounds, &output_directory).await?;

        Ok::<_, anyhow::Error>(())
    };

    let e = match closure.await {
        Err(e) => e,
        Ok(()) => return Ok(()),
    };

    error!("{uuid}: failed on {stage:?}: {e}");
    error!("{uuid}: cleaning up");

    stage = stage.error_variant();
    recording_row.stage = stage as i32;
    recording_row.status = e.to_string();
    alert_clients_of_database_change(clients, &database.update_recording(&recording_row)?).await?;

    remove_dir_all(&output_directory).await?;
    Err(e)
}
