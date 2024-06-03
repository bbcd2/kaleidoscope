use crate::{
    callbacks::alert_clients_of_database_change,
    database::{Database, NewRecording},
    ClientConnections,
};

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use chrono::DateTime;
use log::{error, info};
use tokio::fs::{create_dir_all, remove_dir_all};

const TEMP_DIRECTORY: &str = "temp";

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
    DownloadingFailed = 10,
    CombiningFailed = 11,
    EncodingFailed = 12,
    UploadingFailed = 13,
}
impl Stage {
    pub fn error_variant(self) -> Option<Self> {
        match self {
            Self::Downloading => Some(Self::DownloadingFailed),
            Self::Combining => Some(Self::CombiningFailed),
            Self::Encoding => Some(Self::EncodingFailed),
            Self::Uploading => Some(Self::UploadingFailed),
            _ => None,
        }
    }
}

/// Convert the timestamp to a segment index (referred to in the digest as $Number$)
fn calculate_segment_idx(timestamp: usize) -> usize {
    // <SegmentTemplate ... timescale="50" duration="192" />
    const MAGIC_OFFSET: usize = 38; // 145.92 seconds
    (((timestamp as f64) / (192. / 50.)).floor() as usize) + MAGIC_OFFSET
}

async fn download_segments(
    uuid: &str,
    channel: usize,
    segment_idx_bounds: [usize; 2],
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
    info!("starting clip for {uuid}");

    let mut stage = Stage::Initializing;

    let mut timestamp_bounds = timeframe.iter().map(|bound| -> Result<_> {
        Ok(DateTime::from_timestamp(*bound as i64, 0)
            .context("failed to convert bounds to timestamp")?
            .naive_utc())
    });

    let mut recording_row = NewRecording {
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
        download_segments(&uuid, channel, segment_idx_bounds).await?;

        Ok::<_, anyhow::Error>(())
    };
    match closure.await {
        Err(e) => {
            error!("failed on {stage:?}. cleaning up");

            stage = stage.error_variant().unwrap_or(stage);
            recording_row.stage = stage as i32;
            recording_row.status = e.to_string();
            alert_clients_of_database_change(clients, &database.update_recording(&recording_row)?)
                .await?;

            remove_dir_all(&output_directory).await?;
            Err(e)
        }
        okay => okay,
    }
}
