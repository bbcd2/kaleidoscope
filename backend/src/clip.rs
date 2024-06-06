use crate::{
    consts::{SourceEntry, SOURCES},
    database::{Database, Recording, RecordingUpdate},
    websocket_callbacks::alert_clients_of_database_change,
    ClientConnections,
};

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use chrono::DateTime;
use ffmpeg_cli::{FfmpegBuilder, Parameter};
use futures_util::{
    future::{join_all, try_join_all},
    join, stream, StreamExt, TryStreamExt as _,
};
use log::{debug, error, info, trace};
use serde::Deserialize;
use tokio::{
    fs::{create_dir_all, remove_dir_all, remove_file, File},
    io::AsyncWriteExt as _,
    sync::Mutex,
    time::Instant,
};

const VIDEO_DOWNLOAD_JOB_COUNT: usize = 10;
const TEMP_DIRECTORY: &str = "temp";
const INIT_DIRECTORY: &str = "init";

struct StatusReporter {
    pub clients: ClientConnections,
    pub database: Database,
    pub recording_row: RecordingUpdate,
}
impl StatusReporter {
    pub async fn update(&mut self, status: String, stage: Stage) -> Result<()> {
        debug!(
            "{uuid}: updating row: {stage:?} {status}",
            uuid = &self.recording_row.uuid
        );
        self.recording_row.stage = stage as i32;
        self.recording_row.status = status;
        let recording = self.database.update_recording(&self.recording_row)?;
        alert_clients_of_database_change(self.clients.clone(), &recording).await?;
        Ok(())
    }
}

/// Parameters from the request
#[derive(Deserialize)]
pub struct ClipParameters {
    pub start_timestamp: usize,
    pub end_timestamp: usize,
    pub channel: String,
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

/// Download a url to a path.
/// If the file already exists, the url will not be downloaded.
/// If the download fails, the path will be deleted.
/// Returns whether or not the url was downloaded.
async fn download(url: String, path: &Path) -> Result<bool> {
    if path.exists() {
        return Ok(false);
    }

    if let Some(e) = async move {
        trace!("downloading {url} to {path}", path = path.display());
        let mut resp = reqwest::get(&url)
            .await
            .and_then(|resp| resp.error_for_status())
            .with_context(|| anyhow!("request to {url}"))?;
        let mut output = File::create(&path)
            .await
            .with_context(|| anyhow!("creating file {path}", path = path.display()))?;
        let mut chunk_idx = 0;
        while let Some(chunk) = resp.chunk().await? {
            trace!("{url} chunk {chunk_idx}");
            chunk_idx += 1;
            output.write(&chunk).await.with_context(|| {
                anyhow!("writing chunk from {url} -> {path}", path = path.display())
            })?;
        }
        trace!("{url} done!");
        Ok::<_, anyhow::Error>(())
    }
    .await
    .err()
    {
        let _ = remove_file(path).await;
        Err(e)?;
    }

    Ok(true)
}

async fn download_init_segments(channel: &str, url_prefix: &str) -> Result<()> {
    let video_url = format!("{url_prefix}v=pv14/b=5070016/segment.init");
    let audio_url = format!("{url_prefix}a=pa3/al=en-GB/ap=main/b=96000/segment.init");

    let init_path = PathBuf::new()
        .join(TEMP_DIRECTORY)
        .join(INIT_DIRECTORY)
        .join(channel);
    create_dir_all(&init_path).await?;
    let video_path = init_path.join("video_init.m4s");
    let audio_path = init_path.join("audio_init.m4s");

    download(video_url, &video_path).await?;
    download(audio_url, &audio_path).await?;

    Ok(())
}

/// Download segments to the resultant directory
async fn download_segments(
    uuid: &str,
    channel: &str,
    segment_idx_bounds: [usize; 2],
    target_directory: &Path,
    status_reporter: &mut StatusReporter,
) -> Result<()> {
    status_reporter
        .update("Starting download".to_string(), Stage::Downloading)
        .await?;

    let &SourceEntry { url_prefix, id } = SOURCES.get(channel).context("failed to find channel")?;
    download_init_segments(channel, url_prefix)
        .await
        .context("failed to download initial segments")?;

    let status_reporter = Arc::new(Mutex::new(status_reporter));
    let download_count = Arc::new(AtomicUsize::new(0));
    stream::iter(segment_idx_bounds[0]..=segment_idx_bounds[1])
        .map(|segment_idx| Ok::<_, anyhow::Error>(segment_idx))
        .try_for_each_concurrent(VIDEO_DOWNLOAD_JOB_COUNT, |segment_idx| {
            let status_reporter = status_reporter.clone();
            let download_count = download_count.clone();
            async move {
                let start = Instant::now();
                let progress = format!(
                    "{}/{}",
                    segment_idx - segment_idx_bounds[0] + 1,
                    segment_idx_bounds[1] - segment_idx_bounds[0] + 1
                );
                debug!("{uuid}: downloading segment {progress}");

                let video_url = format!("{url_prefix}t=3840/v=pv14/b=5070016/{segment_idx}.m4s");
                let audio_url =
                    format!("{url_prefix}t=3840/a=pa3/al=en-GB/ap=main/b=96000/{segment_idx}.m4s");

                let base_path = PathBuf::new().join(TEMP_DIRECTORY).join(uuid);
                let video_path = base_path.join(format!("video_{segment_idx}.m4s"));
                let audio_path = base_path.join(format!("audio_{segment_idx}.m4s"));

                if let Some(e) = <(_, _) as Into<[Result<_>; 2]>>::into(join!(
                    download(video_url, &video_path),
                    download(audio_url, &audio_path)
                ))
                .into_iter()
                .find_map(|result| result.err())
                {
                    Err(e)?;
                }

                let duration_sec = Instant::now().duration_since(start).as_secs();
                let count = download_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                let progress = format!(
                    "{count}/{} total segments (last finished: {progress} in {duration_sec} sec)",
                    segment_idx_bounds[1] - segment_idx_bounds[0] + 1
                );
                let mut status_reporter = status_reporter.lock().await;
                status_reporter
                    .update(format!("Downloaded {progress}"), Stage::Downloading)
                    .await?;

                Ok(())
            }
        })
        .await?;

    Ok(())
}

async fn combine_segments(
    status_reporter: &mut StatusReporter,
    segment_idx_bounds: [usize; 2],
    uuid: &str,
    channel: &str,
    encode: bool,
) -> Result<()> {
    status_reporter
        .update(format!("Starting segment combination"), Stage::Combining)
        .await?;

    // Helper to get an FFmpeg builder with default options
    let get_default_builder = || {
        FfmpegBuilder::new()
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .option(Parameter::Single("nostdin"))
            .option(Parameter::Single("y")) // overwrite output files
    };

    let job_path = PathBuf::new().join(TEMP_DIRECTORY).join(uuid);
    let init_path = PathBuf::new()
        .join(TEMP_DIRECTORY)
        .join(INIT_DIRECTORY)
        .join(channel);

    let output_path = job_path.join("output.mp4");
    let video_concat_path = job_path.join("video_full.mp4");
    let audio_concat_path = job_path.join("audio_full.mp4");

    // Get concatenation inputs (`-i "concat:{init}|{segment 1}...|{segment n}"`)
    let [video_concat_input, audio_concat_input] = ["video", "audio"].map(|r#type| {
        let init = init_path.join(&format!("{type}_init.m4s"));
        let mut res = format!("concat:{}", init.to_string_lossy().to_string());
        for path in (segment_idx_bounds[0]..=segment_idx_bounds[1])
            .map(|segment_idx| job_path.join(&format!("{type}_{segment_idx}.m4s")))
        {
            res.push_str(&format!("|{}", path.to_string_lossy()));
        }
        res
    });
    debug!("video concat:\n{video_concat_input}\naudio concat:\n{audio_concat_input}");

    // FFmpeg concatenation commands
    let [video_concat, audio_concat] = [
        (&video_concat_input, video_concat_path.to_str().unwrap()),
        (&audio_concat_input, audio_concat_path.to_str().unwrap()),
    ]
    .map(|(input, output)| async move {
        debug!("creating ffmpeg builder for {input} -> {output}");
        let builder = get_default_builder()
            .input(ffmpeg_cli::File::new(input))
            .output(ffmpeg_cli::File::new(output).option(Parameter::KeyValue("c", "copy")));
        builder.run().await.map_err(|e| anyhow!("{e}"))
    });

    let length = Duration::from_secs(100);

    video_concat.await?.process.wait_with_output()?;
    audio_concat.await?.process.wait_with_output()?;

    // Combine concatenated audio and video
    let combine = get_default_builder()
        .input(ffmpeg_cli::File::new(audio_concat_path.to_str().unwrap()))
        .input(ffmpeg_cli::File::new(video_concat_path.to_str().unwrap()))
        .output(
            ffmpeg_cli::File::new(output_path.to_str().unwrap())
                .option(Parameter::KeyValue("c:a", "copy"))
                .option(Parameter::KeyValue(
                    "c:v",
                    if encode { "libx264" } else { "copy" },
                )),
        );
    let mut combine_job = combine
        .run()
        .await
        .map_err(|e| anyhow!("failed to combine: {e}"))?;
    debug!("running combine job");

    // Report progress
    while let Some(progress) = combine_job.progress.next().await {
        if let Ok(progress) = progress {
            let percentage = ((progress.out_time.unwrap().as_secs() as f32)
                / (length.as_secs() as f32)
                * 100.) as usize;
            status_reporter
                .update(format!("Combining: {percentage}%"), Stage::Combining)
                .await?;
        }
    }

    combine_job.process.wait_with_output()?;

    Ok(())
}

pub async fn clip(
    clients: ClientConnections,
    uuid: String,
    channel: String,
    timeframe: [usize; 2],
    encode: bool,
    database: Database,
) -> Result<()> {
    info!("{uuid}: starting clip");

    let mut timestamp_bounds = timeframe.iter().map(|bound| -> Result<_> {
        Ok(DateTime::from_timestamp(*bound as i64, 0)
            .context("failed to convert bounds to timestamp")?
            .naive_utc())
    });

    let mut status_reporter = StatusReporter {
        clients,
        recording_row: RecordingUpdate {
            user_id: None,
            rec_start: timestamp_bounds.next().unwrap()?,
            rec_end: timestamp_bounds.next().unwrap()?,
            stage: Stage::Initializing as i32,
            status: "Pending".to_string(),
            uuid: uuid.clone(),
            channel: channel.clone(),
        },
        database,
    };

    status_reporter
        .database
        .create_recording(&status_reporter.recording_row)?;

    let output_directory = PathBuf::from(TEMP_DIRECTORY).join(&uuid);

    let result = async {
        let segment_idx_bounds = timeframe.map(|bound| calculate_segment_idx(bound));

        create_dir_all(&output_directory).await?;
        download_segments(
            &uuid,
            &channel,
            segment_idx_bounds,
            &output_directory,
            &mut status_reporter,
        )
        .await?;
        combine_segments(
            &mut status_reporter,
            segment_idx_bounds,
            &uuid,
            &channel,
            encode,
        )
        .await?;

        Ok::<_, anyhow::Error>(())
    }
    .await;

    let e = match result {
        Ok(_) => {
            info!("{uuid}: done");
            return Ok(());
        }
        Err(e) => e,
    };

    error!("{uuid}: failed: {e:?}");
    error!("{uuid}: cleaning up");

    remove_dir_all(&output_directory).await?;
    Err(e)
}
