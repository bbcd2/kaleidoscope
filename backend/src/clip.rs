use crate::{
    consts::{SourceEntry, SOURCES, WEBDAV_PASSWORD, WEBDAV_URL, WEBDAV_USERNAME},
    database::{Database, Recording, RecordingUpdate, Uuid},
    websocket_callbacks::alert_clients_of_database_change,
    ClientConnections, PORT,
};

use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{atomic::AtomicUsize, Arc},
};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, TimeDelta};
use ffmpeg_cli::{FfmpegBuilder, Parameter};
use futures_util::{join, stream, StreamExt, TryStreamExt as _};
use lazy_static::lazy_static;
use log::{debug, error, info, trace};
use serde::Deserialize;
use tokio::{
    fs::{create_dir_all, remove_dir_all, remove_file, File},
    io::AsyncWriteExt as _,
    process::Command,
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        Mutex, RwLock,
    },
    time::{sleep, Instant},
};
use warp::reply::Reply;

/// `None` signifies the end of an FFmpeg job
pub type FfmpegProgressChannels = Arc<RwLock<HashMap<Uuid, UnboundedSender<Option<TimeDelta>>>>>;
enum JobQueueReport {
    Ready,
    InQueue(usize),
}
type JobQueueChannel = RwLock<VecDeque<(Uuid, UnboundedSender<JobQueueReport>)>>;

const VIDEO_DOWNLOAD_JOB_COUNT: usize = 10;
const TEMP_DIRECTORY: &str = "temp";
const INIT_DIRECTORY: &str = "init";

lazy_static! {
    static ref JOB_QUEUE_CHANNEL: JobQueueChannel = RwLock::new(VecDeque::new());
}
const MAX_JOB_COUNT: usize = 1;

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
        self.alert(&recording).await?;
        Ok(())
    }
    pub async fn alert(&mut self, recording: &Recording) -> Result<()> {
        alert_clients_of_database_change(self.clients.clone(), recording).await
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
#[repr(i32)]
pub enum Stage {
    WaitingQueue = 0,
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

pub async fn ffmpeg_progress_update_handler(
    mut body: impl warp::Stream<Item = Result<impl warp::Buf, warp::Error>> + Unpin + Send + Sync,
    uuid: Uuid,
    ffmpeg_progress_channels: FfmpegProgressChannels,
) -> Result<impl Reply> {
    let find_out_time = |line: &str| {
        line.split_once("out_time_us=")
            .and_then(|(_, time)| Some(TimeDelta::milliseconds(time.parse::<i64>().ok()? / 1000)))
    };

    let lookup = ffmpeg_progress_channels.read().await;
    let tx = lookup
        .get(&uuid)
        .ok_or(anyhow!("ffmpeg response uuid {uuid} not found"))?;

    while let Some(buf) = body.next().await {
        let mut buf = buf.context("failed to get buffer")?;
        while buf.remaining() > 0 {
            let chunk = buf.chunk();
            let chunk_len = chunk.len();
            let lines = std::str::from_utf8(chunk).unwrap_or_default().lines();
            for line in lines {
                if let Some(out_time) = find_out_time(line) {
                    trace!("ffmpeg progress out time: {out_time}");
                    tx.send(Some(out_time))?;
                }
            }
            buf.advance(chunk_len);
        }
    }

    tx.send(None)?;

    Ok(warp::reply())
}

async fn wait_in_queue(status_reporter: &mut StatusReporter, uuid: Uuid) -> Result<()> {
    let (tx, mut rx) = unbounded_channel();
    let len = {
        let mut queue_channels = JOB_QUEUE_CHANNEL.write().await;
        queue_channels.push_back((uuid.clone(), tx));
        queue_channels.len()
    };
    if len > MAX_JOB_COUNT {
        loop {
            if let Some(JobQueueReport::InQueue(queue_pos)) = rx.recv().await {
                info!("{uuid}: waiting in queue pos {queue_pos}");
                status_reporter
                    .update(format!("Queue position: {queue_pos}"), Stage::WaitingQueue)
                    .await?;
                continue;
            }
            break;
        }
    }
    info!("{uuid}: queue ready!"); // debugify
    Ok(())
}
async fn advance_queue(pop_uuid: Uuid) -> Result<()> {
    info!("{pop_uuid}: advancing queue"); // debugify

    let mut queue_channels = JOB_QUEUE_CHANNEL.write().await;
    let pop_idx = queue_channels
        .iter()
        .enumerate()
        .find_map(|(idx, (uuid, _))| if uuid == &pop_uuid { Some(idx) } else { None })
        .context(anyhow!("failed to pop uuid {pop_uuid}"))?;
    queue_channels.remove(pop_idx);

    // We just mutated the queue to remove ourself, so the index is subtracted by 1
    for idx in (MAX_JOB_COUNT - 1)..queue_channels.len() {
        info!("{pop_uuid}: alerting queue pos {idx}"); // debugify
        let (_, update) = &queue_channels[idx];
        update.send(if idx == (MAX_JOB_COUNT - 1) {
            JobQueueReport::Ready
        } else {
            JobQueueReport::InQueue(idx)
        })?;
    }

    Ok(())
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
    status_reporter: &mut StatusReporter,
    uuid: &str,
    channel: &str,
    segment_idx_bounds: [usize; 2],
    target_directory: &Path,
) -> Result<()> {
    status_reporter
        .update("Starting download".to_string(), Stage::Downloading)
        .await?;

    let &SourceEntry { url_prefix, .. } = SOURCES.get(channel).context("failed to find channel")?;
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

                let base_path = PathBuf::from(target_directory);
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
    ffmpeg_progress_channels: FfmpegProgressChannels,
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

    // Create progress channel
    let (tx, mut progress_rx) = unbounded_channel();
    {
        let mut lookup = ffmpeg_progress_channels.write().await;
        lookup.insert(uuid.to_string(), tx);
    }

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

    // Video and audio concatenation
    let [video_concat, audio_concat] = [
        (&video_concat_input, video_concat_path.to_str().unwrap()),
        (&audio_concat_input, audio_concat_path.to_str().unwrap()),
    ]
    .map(|(input, output)| {
        get_default_builder()
            .input(ffmpeg_cli::File::new(input))
            .output(ffmpeg_cli::File::new(output).option(Parameter::KeyValue("c", "copy")))
    });

    let video_concat_job = Command::from(
        video_concat
            .option(Parameter::KeyValue(
                "progress",
                &format!("http://127.0.0.1:{PORT}/ffmpeg-progress/{uuid}"),
            ))
            .to_command(),
    )
    .spawn()
    .context("spawning video concat command")?;
    let audio_concat_job = Command::from(audio_concat.to_command())
        .spawn()
        .context("spawning audio concat command")?;

    status_reporter
        .update(
            format!("Concatenating audio and video segments"),
            Stage::Combining,
        )
        .await?;

    video_concat_job.wait_with_output().await?;
    audio_concat_job.wait_with_output().await?;
    let mut time = None;
    while let Some(Some(progress)) = progress_rx.recv().await {
        time = Some(progress);
    }
    let time = time.unwrap_or_default();

    // Combine concatenated audio and video
    status_reporter
        .update(format!("Combining and encoding segments"), Stage::Encoding)
        .await?;
    let mut combine_job = get_default_builder()
        .input(ffmpeg_cli::File::new(audio_concat_path.to_str().unwrap()))
        .input(ffmpeg_cli::File::new(video_concat_path.to_str().unwrap()))
        .output(
            ffmpeg_cli::File::new(output_path.to_str().unwrap())
                .option(Parameter::KeyValue("c:a", "copy"))
                .option(Parameter::KeyValue(
                    "c:v",
                    if encode { "libx264" } else { "copy" },
                )),
        )
        .option(Parameter::KeyValue(
            "progress",
            &format!("http://127.0.0.1:{PORT}/ffmpeg-progress/{uuid}"),
        ))
        .to_command()
        .spawn()
        .context("spawning combine job")?;
    sleep(TimeDelta::seconds(1).to_std()?).await;
    if let Some(status) = combine_job.try_wait()? {
        if !status.success() {
            Err(anyhow!("combine job failed! {:?}", combine_job.stderr))?;
        }
    }
    while let Some(maybe_progress) = progress_rx.recv().await {
        let progress = match maybe_progress {
            Some(progress) => progress,
            None => break,
        };
        let percentage = (progress.num_seconds() as f32 / time.num_seconds() as f32) * 100.;
        status_reporter
            .update(
                format!("Combining and encoding segments ({percentage:.2}%)"),
                Stage::Encoding,
            )
            .await?;
    }

    Ok(())
}

async fn upload(status_reporter: &mut StatusReporter, uuid: &str) -> Result<()> {
    status_reporter
        .update("Uploading result".to_string(), Stage::Uploading)
        .await?;

    let output_path = PathBuf::new()
        .join(TEMP_DIRECTORY)
        .join(uuid)
        .join("output.mp4");

    Command::new("curl")
        .args([
            "-T",
            output_path.to_str().unwrap(),
            "-u",
            &format!(
                "{WEBDAV_USERNAME}:{WEBDAV_PASSWORD}",
                WEBDAV_USERNAME = WEBDAV_USERNAME.to_string(),
                WEBDAV_PASSWORD = WEBDAV_PASSWORD.to_string()
            ),
            &format!(
                "{WEBDAV_URL}/bbcd/{uuid}.mp4",
                WEBDAV_URL = WEBDAV_URL.to_string()
            ),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?
        .wait_with_output()
        .await?;

    Ok(())
}

pub async fn clip(
    uuid: String,
    channel: String,
    timeframe: [usize; 2],
    encode: bool,
    database: Database,
    clients: ClientConnections,
    ffmpeg_progress_channels: FfmpegProgressChannels,
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
            stage: 0,
            status: "Pending".to_string(),
            uuid: uuid.clone(),
            channel: channel.clone(),
        },
        database,
    };
    let recording = status_reporter
        .database
        .create_recording(&status_reporter.recording_row)?;
    status_reporter.alert(&recording).await?;

    let output_directory = PathBuf::from(TEMP_DIRECTORY).join(&uuid);
    let segment_idx_bounds = timeframe.map(|bound| calculate_segment_idx(bound));

    let result = async {
        wait_in_queue(&mut status_reporter, uuid.to_string()).await?;
        create_dir_all(&output_directory).await?;
        download_segments(
            &mut status_reporter,
            &uuid,
            &channel,
            segment_idx_bounds,
            &output_directory,
        )
        .await?;
        combine_segments(
            &mut status_reporter,
            segment_idx_bounds,
            &uuid,
            &channel,
            encode,
            ffmpeg_progress_channels,
        )
        .await?;
        upload(&mut status_reporter, &uuid).await?;

        Ok::<_, anyhow::Error>(())
    }
    .await;

    advance_queue(uuid.clone()).await?;

    let e = match result {
        Ok(_) => {
            status_reporter
                .update("Done".to_string(), Stage::Complete)
                .await?;
            info!("{uuid}: done");
            return Ok(());
        }
        Err(e) => e,
    };

    status_reporter
        .update(
            format!("{e:?}"),
            Stage::error_variant(unsafe {
                std::mem::transmute(status_reporter.recording_row.stage)
            }),
        )
        .await?;

    error!("{uuid}: failed: {e:?}");
    error!("{uuid}: cleaning up");

    remove_dir_all(&output_directory).await?;
    Err(e)
}
