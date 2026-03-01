use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_config::meta::region::RegionProviderChain;
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client, primitives::ByteStream};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UploadTargetKind {
    AwsS3,
}

impl fmt::Display for UploadTargetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AwsS3 => write!(f, "AWS S3"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UploadOverwriteMode {
    #[default]
    Overwrite,
    SkipExisting,
}

impl UploadOverwriteMode {
    pub const ALL: [Self; 2] = [Self::Overwrite, Self::SkipExisting];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Overwrite => "overwrite",
            Self::SkipExisting => "skip_existing",
        }
    }

    pub(crate) fn from_str(value: &str) -> Self {
        if value == "skip_existing" {
            Self::SkipExisting
        } else {
            Self::Overwrite
        }
    }
}

impl fmt::Display for UploadOverwriteMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overwrite => write!(f, "Overwrite existing"),
            Self::SkipExisting => write!(f, "Skip existing"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct UploadProgress {
    pub uploaded_files: usize,
    pub skipped_files: usize,
    pub failed_files: usize,
    pub total_files: usize,
    pub uploaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: f32,
}

#[derive(Debug, Clone)]
pub(crate) enum UploadWorkerEvent {
    Started {
        total_files: usize,
        total_bytes: u64,
    },
    LogLine(String),
    Progress(UploadProgress),
    Finished {
        summary: UploadProgress,
        was_canceled: bool,
    },
    SpawnError(String),
}

#[derive(Debug, Clone)]
pub(crate) struct S3UploadRequest {
    pub local_dir: PathBuf,
    pub bucket: String,
    pub prefix: String,
    pub region: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub overwrite_mode: UploadOverwriteMode,
}

#[derive(Debug, Clone)]
struct UploadFile {
    path: PathBuf,
    size: u64,
}

pub(crate) fn start_s3_upload_worker(
    request: S3UploadRequest,
    sender: mpsc::Sender<UploadWorkerEvent>,
    cancel_flag: Arc<AtomicBool>,
) {
    thread::spawn(move || run_s3_upload_worker(request, sender, cancel_flag));
}

fn run_s3_upload_worker(
    request: S3UploadRequest,
    sender: mpsc::Sender<UploadWorkerEvent>,
    cancel_flag: Arc<AtomicBool>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            let _ = sender.send(UploadWorkerEvent::SpawnError(format!(
                "Could not initialize async upload runtime: {err}"
            )));
            return;
        }
    };

    if let Err(err) = runtime.block_on(upload_s3_folder(request, &sender, &cancel_flag)) {
        let _ = sender.send(UploadWorkerEvent::SpawnError(err.to_string()));
    }
}

async fn upload_s3_folder(
    request: S3UploadRequest,
    sender: &mpsc::Sender<UploadWorkerEvent>,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<()> {
    let files = collect_upload_files(&request.local_dir)?;
    let total_bytes = files.iter().map(|file| file.size).sum::<u64>();

    let _ = sender.send(UploadWorkerEvent::Started {
        total_files: files.len(),
        total_bytes,
    });

    if files.is_empty() {
        let _ = sender.send(UploadWorkerEvent::LogLine(
            "No files found in output folder. Nothing to upload.".to_string(),
        ));
        let _ = sender.send(UploadWorkerEvent::Finished {
            summary: UploadProgress::default(),
            was_canceled: false,
        });
        return Ok(());
    }

    let region_provider = if let Some(region) = non_empty(request.region.as_deref()) {
        RegionProviderChain::first_try(aws_config::Region::new(region))
            .or_default_provider()
            .or_else("eu-north-1")
    } else {
        RegionProviderChain::default_provider().or_else("eu-north-1")
    };

    let mut loader = aws_config::defaults(BehaviorVersion::latest()).region(region_provider);
    if let (Some(access_key_id), Some(secret_access_key)) = (
        non_empty(request.access_key_id.as_deref()),
        non_empty(request.secret_access_key.as_deref()),
    ) {
        let credentials = Credentials::new(
            access_key_id,
            secret_access_key,
            non_empty(request.session_token.as_deref()),
            None,
            "hlsenpai-config",
        );
        loader = loader.credentials_provider(credentials);
    }
    let aws_config = loader.load().await;
    let client = Client::new(&aws_config);

    let mut progress = UploadProgress {
        total_files: files.len(),
        total_bytes,
        ..UploadProgress::default()
    };

    for file in files {
        if cancel_flag.load(Ordering::Relaxed) {
            let _ = sender.send(UploadWorkerEvent::LogLine(
                "Upload canceled by user.".to_string(),
            ));
            let _ = sender.send(UploadWorkerEvent::Finished {
                summary: progress,
                was_canceled: true,
            });
            return Ok(());
        }

        let key = to_s3_key(&request.local_dir, &file.path, &request.prefix)?;

        if request.overwrite_mode == UploadOverwriteMode::SkipExisting {
            match client
                .head_object()
                .bucket(&request.bucket)
                .key(&key)
                .send()
                .await
            {
                Ok(_) => {
                    progress.skipped_files += 1;
                    emit_progress(sender, &mut progress);
                    let _ = sender.send(UploadWorkerEvent::LogLine(format!(
                        "Skipped existing object: s3://{}/{}",
                        request.bucket, key
                    )));
                    continue;
                }
                Err(err) => {
                    let text = err.to_string();
                    if !text.contains("NotFound") && !text.contains("404") {
                        let _ = sender.send(UploadWorkerEvent::LogLine(format!(
                            "HEAD check failed for s3://{}/{}: {} (continuing with upload)",
                            request.bucket, key, text
                        )));
                    }
                }
            }
        }

        match upload_file(&client, &request.bucket, &key, &file.path).await {
            Ok(()) => {
                progress.uploaded_files += 1;
                progress.uploaded_bytes = progress.uploaded_bytes.saturating_add(file.size);
                let _ = sender.send(UploadWorkerEvent::LogLine(format!(
                    "Uploaded: s3://{}/{}",
                    request.bucket, key
                )));
            }
            Err(err) => {
                progress.failed_files += 1;
                let _ = sender.send(UploadWorkerEvent::LogLine(format!(
                    "Failed: s3://{}/{} ({})",
                    request.bucket, key, err
                )));
            }
        }

        emit_progress(sender, &mut progress);
    }

    let _ = sender.send(UploadWorkerEvent::Finished {
        summary: progress,
        was_canceled: false,
    });
    Ok(())
}

fn collect_upload_files(local_dir: &Path) -> Result<Vec<UploadFile>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(local_dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path().to_path_buf();
        let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        files.push(UploadFile { path, size });
    }

    files.sort_by_key(|file| {
        let is_playlist = file
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("m3u8"));
        if is_playlist {
            (1_u8, file.path.clone())
        } else {
            (0_u8, file.path.clone())
        }
    });

    Ok(files)
}

fn emit_progress(sender: &mpsc::Sender<UploadWorkerEvent>, progress: &mut UploadProgress) {
    let completed = progress
        .uploaded_files
        .saturating_add(progress.skipped_files)
        .saturating_add(progress.failed_files);
    if progress.total_files > 0 {
        progress.percent = ((completed as f64 / progress.total_files as f64) * 100.0) as f32;
    }
    let _ = sender.send(UploadWorkerEvent::Progress(progress.clone()));
}

fn content_type_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "m3u8" => "application/vnd.apple.mpegurl",
        "ts" => "video/mp2t",
        "vtt" => "text/vtt",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        _ => "application/octet-stream",
    }
}

fn cache_control_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "m3u8" => "public, max-age=60",
        "ts" => "public, max-age=31536000, immutable",
        _ => "public, max-age=3600",
    }
}

fn to_s3_key(root: &Path, file: &Path, prefix: &str) -> Result<String> {
    let relative = file
        .strip_prefix(root)
        .context("file is not under output root")?;

    let relative_path = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/");

    let prefix = prefix.trim().trim_matches('/');
    if prefix.is_empty() {
        Ok(relative_path)
    } else {
        Ok(format!("{prefix}/{relative_path}"))
    }
}

async fn upload_file(client: &Client, bucket: &str, key: &str, path: &Path) -> Result<()> {
    let body = ByteStream::from_path(path.to_path_buf())
        .await
        .with_context(|| format!("reading {}", path.display()))?;

    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .content_type(content_type_for(path))
        .cache_control(cache_control_for(path))
        .body(body)
        .send()
        .await
        .with_context(|| format!("put_object s3://{bucket}/{key}"))?;

    Ok(())
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
