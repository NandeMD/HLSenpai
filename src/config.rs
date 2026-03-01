use crate::app::{
    AudioCodec, EncodeOptionsForm, HlsPlaylistType, VariantForm, VideoCodecLib, VideoProfile,
    X264Preset,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CONFIG_VERSION: u32 = 1;

#[derive(Debug)]
pub(crate) enum ConfigError {
    Io(std::io::Error),
    Json(serde_json::Error),
    ProjectDirUnavailable,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Json(err) => write!(f, "JSON error: {err}"),
            Self::ProjectDirUnavailable => {
                write!(f, "Could not resolve an OS-specific config directory")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ConfigPaths {
    pub dir: PathBuf,
    pub auth_file: PathBuf,
    pub preset_file: PathBuf,
    pub upload_prefs_file: PathBuf,
}

impl ConfigPaths {
    pub(crate) fn discover() -> Result<Self, ConfigError> {
        if let Some(xdg_dir) = env::var_os("XDG_CONFIG_HOME") {
            return Ok(Self::from_dir(PathBuf::from(xdg_dir).join("hlsenpai")));
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(app_data) = env::var_os("APPDATA") {
                return Ok(Self::from_dir(PathBuf::from(app_data).join("hlsenpai")));
            }
        }

        if let Some(home) = env::var_os("HOME") {
            return Ok(Self::from_dir(
                PathBuf::from(home).join(".config").join("hlsenpai"),
            ));
        }

        Err(ConfigError::ProjectDirUnavailable)
    }

    pub(crate) fn fallback_current_dir() -> Self {
        Self::from_dir(PathBuf::from("."))
    }

    pub(crate) fn from_dir(dir: PathBuf) -> Self {
        Self {
            auth_file: dir.join("auth_config.json"),
            preset_file: dir.join("encoding_presets.json"),
            upload_prefs_file: dir.join("upload_prefs.json"),
            dir,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub(crate) struct AuthProviderConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub extra: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct AppAuthConfig {
    #[serde(default = "default_config_version")]
    pub version: u32,
    #[serde(default)]
    pub providers: BTreeMap<String, AuthProviderConfig>,
    #[serde(default)]
    pub updated_at_utc: Option<String>,
}

impl Default for AppAuthConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            providers: BTreeMap::new(),
            updated_at_utc: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LastEncodingPresetConfig {
    #[serde(default = "default_config_version")]
    version: u32,
    preset: PersistedEncodePreset,
    #[serde(default)]
    updated_at_utc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct PersistedEncodePreset {
    pub scale_width: u32,
    pub scale_height: u32,
    pub scale_lock_aspect: bool,
    pub gop: u32,
    pub video_codec_lib: VideoCodecLib,
    pub profile: VideoProfile,
    pub preset: X264Preset,
    pub sc_threshold: i32,
    pub audio_codec: AudioCodec,
    pub audio_channels: u8,
    pub hls_time_seconds: u32,
    pub hls_playlist_type: HlsPlaylistType,
    pub hls_flags_independent_segments: bool,
    pub master_playlist_name: String,
    pub segment_filename_pattern: String,
    pub output_variant_playlist_pattern: String,
    pub output_base_folder: String,
    pub output_subfolder_name: String,
    pub output_master_playlist_file: String,
    #[serde(default)]
    pub variants: Vec<VariantForm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub(crate) struct PersistedUploadPrefs {
    #[serde(default = "default_config_version")]
    pub version: u32,
    #[serde(default)]
    pub last_upload_provider: Option<String>,
    #[serde(default)]
    pub last_overwrite_mode: Option<String>,
    #[serde(default)]
    pub last_bucket: Option<String>,
    #[serde(default)]
    pub last_prefix: Option<String>,
    #[serde(default)]
    pub updated_at_utc: Option<String>,
}

impl PersistedEncodePreset {
    pub(crate) fn from_form(form: &EncodeOptionsForm) -> Self {
        Self {
            scale_width: form.scale_width,
            scale_height: form.scale_height,
            scale_lock_aspect: form.scale_lock_aspect,
            gop: form.gop,
            video_codec_lib: form.video_codec_lib,
            profile: form.profile,
            preset: form.preset,
            sc_threshold: form.sc_threshold,
            audio_codec: form.audio_codec,
            audio_channels: form.audio_channels,
            hls_time_seconds: form.hls_time_seconds,
            hls_playlist_type: form.hls_playlist_type,
            hls_flags_independent_segments: form.hls_flags_independent_segments,
            master_playlist_name: form.master_playlist_name.clone(),
            segment_filename_pattern: form.segment_filename_pattern.clone(),
            output_variant_playlist_pattern: form.output_variant_playlist_pattern.clone(),
            output_base_folder: form.output_base_folder.clone(),
            output_subfolder_name: form.output_subfolder_name.clone(),
            output_master_playlist_file: form.output_master_playlist_file.clone(),
            variants: form.variants.to_vec(),
        }
    }

    fn normalize_in_place(&mut self) {
        self.scale_width = self.scale_width.max(1);
        self.scale_height = self.scale_height.max(1);
        self.gop = self.gop.max(1);
        self.sc_threshold = self.sc_threshold.max(0);
        self.audio_channels = self.audio_channels.max(1);
        self.hls_time_seconds = self.hls_time_seconds.max(1);
        self.variants = normalize_variant_list(&self.variants).to_vec();
    }
}

pub(crate) fn load_auth_config(paths: &ConfigPaths) -> Result<AppAuthConfig, ConfigError> {
    let content = match fs::read_to_string(&paths.auth_file) {
        Ok(content) => content,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(AppAuthConfig::default()),
        Err(err) => return Err(ConfigError::Io(err)),
    };

    let mut cfg: AppAuthConfig = match serde_json::from_str(&content) {
        Ok(cfg) => cfg,
        Err(err) => {
            backup_broken_file(&paths.auth_file);
            eprintln!(
                "Invalid auth config JSON at {}: {}",
                paths.auth_file.display(),
                err
            );
            return Ok(AppAuthConfig::default());
        }
    };

    if cfg.version == 0 {
        cfg.version = CONFIG_VERSION;
    }

    Ok(cfg)
}

pub(crate) fn save_auth_config(
    paths: &ConfigPaths,
    cfg: &AppAuthConfig,
) -> Result<(), ConfigError> {
    let mut to_write = cfg.clone();
    to_write.version = CONFIG_VERSION;
    to_write.updated_at_utc = Some(now_utc_unix_string());

    write_json_atomic(&paths.dir, &paths.auth_file, &to_write)?;
    set_strict_permissions(&paths.auth_file);
    Ok(())
}

pub(crate) fn load_last_preset(
    paths: &ConfigPaths,
) -> Result<Option<PersistedEncodePreset>, ConfigError> {
    let content = match fs::read_to_string(&paths.preset_file) {
        Ok(content) => content,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(ConfigError::Io(err)),
    };

    let parsed: LastEncodingPresetConfig = match serde_json::from_str(&content) {
        Ok(parsed) => parsed,
        Err(err) => {
            backup_broken_file(&paths.preset_file);
            eprintln!(
                "Invalid preset config JSON at {}: {}",
                paths.preset_file.display(),
                err
            );
            return Ok(None);
        }
    };

    let _version = parsed.version.max(1);
    let _updated_at = parsed.updated_at_utc;

    let mut preset = parsed.preset;
    preset.normalize_in_place();
    Ok(Some(preset))
}

pub(crate) fn save_last_preset(
    paths: &ConfigPaths,
    preset: &PersistedEncodePreset,
) -> Result<(), ConfigError> {
    let mut normalized = preset.clone();
    normalized.normalize_in_place();

    let cfg = LastEncodingPresetConfig {
        version: CONFIG_VERSION,
        preset: normalized,
        updated_at_utc: Some(now_utc_unix_string()),
    };

    write_json_atomic(&paths.dir, &paths.preset_file, &cfg)?;
    Ok(())
}

pub(crate) fn load_upload_prefs(paths: &ConfigPaths) -> Result<PersistedUploadPrefs, ConfigError> {
    let content = match fs::read_to_string(&paths.upload_prefs_file) {
        Ok(content) => content,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(PersistedUploadPrefs::default()),
        Err(err) => return Err(ConfigError::Io(err)),
    };

    let mut prefs: PersistedUploadPrefs = match serde_json::from_str(&content) {
        Ok(prefs) => prefs,
        Err(err) => {
            backup_broken_file(&paths.upload_prefs_file);
            eprintln!(
                "Invalid upload prefs JSON at {}: {}",
                paths.upload_prefs_file.display(),
                err
            );
            return Ok(PersistedUploadPrefs::default());
        }
    };

    if prefs.version == 0 {
        prefs.version = CONFIG_VERSION;
    }

    Ok(prefs)
}

pub(crate) fn save_upload_prefs(
    paths: &ConfigPaths,
    prefs: &PersistedUploadPrefs,
) -> Result<(), ConfigError> {
    let mut to_write = prefs.clone();
    to_write.version = CONFIG_VERSION;
    to_write.updated_at_utc = Some(now_utc_unix_string());
    write_json_atomic(&paths.dir, &paths.upload_prefs_file, &to_write)?;
    Ok(())
}

pub(crate) fn apply_preset_to_form(form: &mut EncodeOptionsForm, preset: &PersistedEncodePreset) {
    let mut normalized = preset.clone();
    normalized.normalize_in_place();

    form.scale_lock_aspect = normalized.scale_lock_aspect;
    form.scale_width = normalized.scale_width;
    form.scale_height = normalized.scale_height;
    form.gop = normalized.gop;

    form.video_codec_lib = normalized.video_codec_lib;
    form.profile = if form.profile_options().contains(&normalized.profile) {
        normalized.profile
    } else {
        form.video_codec_lib.default_profile()
    };
    form.preset = if form.preset_options().contains(&normalized.preset) {
        normalized.preset
    } else {
        form.video_codec_lib.default_preset()
    };
    form.sc_threshold = normalized.sc_threshold;

    form.audio_codec = normalized.audio_codec;
    form.audio_channels = normalized.audio_channels.max(1);
    form.hls_time_seconds = normalized.hls_time_seconds.max(1);
    form.hls_playlist_type = normalized.hls_playlist_type;
    form.hls_flags_independent_segments = normalized.hls_flags_independent_segments;
    form.master_playlist_name = normalized.master_playlist_name;
    form.segment_filename_pattern = normalized.segment_filename_pattern;
    form.output_variant_playlist_pattern = normalized.output_variant_playlist_pattern;
    form.output_base_folder = normalized.output_base_folder;
    form.output_subfolder_name = normalized.output_subfolder_name;
    form.output_master_playlist_file = normalized.output_master_playlist_file;
    form.variants = normalize_variant_list(&normalized.variants);
}

fn normalize_variant_list(variants: &[VariantForm]) -> [VariantForm; 3] {
    let defaults = VariantForm::defaults();
    let mut normalized = defaults.clone();

    for index in 0..normalized.len() {
        if let Some(value) = variants.get(index) {
            let mut variant = value.clone();
            if variant.name.trim().is_empty() {
                variant.name = defaults[index].name.clone();
            }
            variant.video_bitrate_k = variant.video_bitrate_k.max(1);
            variant.maxrate_k = variant.maxrate_k.max(1);
            variant.bufsize_k = variant.bufsize_k.max(1);
            variant.audio_bitrate_k = variant.audio_bitrate_k.max(1);
            normalized[index] = variant;
        }
    }

    normalized
}

fn write_json_atomic<T: Serialize>(
    config_dir: &Path,
    destination: &Path,
    value: &T,
) -> Result<(), ConfigError> {
    fs::create_dir_all(config_dir)?;

    let payload = serde_json::to_vec_pretty(value)?;
    let nonce = now_unix_nanos();
    let temp_path = destination.with_extension(format!("tmp-{nonce}"));

    {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(&payload)?;
        file.flush()?;
        file.sync_all()?;
    }

    fs::rename(&temp_path, destination)?;
    Ok(())
}

fn backup_broken_file(path: &Path) {
    if !path.exists() {
        return;
    }

    let broken_path = path.with_extension(format!("broken-{}.json", now_unix_nanos()));
    if let Err(err) = fs::rename(path, &broken_path) {
        eprintln!(
            "Failed to backup broken config file {} -> {}: {}",
            path.display(),
            broken_path.display(),
            err
        );
    }
}

fn default_config_version() -> u32 {
    CONFIG_VERSION
}

fn now_utc_unix_string() -> String {
    format!("{}", now_unix_nanos() / 1_000_000_000)
}

fn now_unix_nanos() -> u128 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_nanos(),
        Err(_) => 0,
    }
}

fn set_strict_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if let Err(err) = fs::set_permissions(path, fs::Permissions::from_mode(0o600)) {
            eprintln!(
                "Could not set strict permissions on auth config {}: {}",
                path.display(),
                err
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ff_helpers::VideoMetadata;

    fn unique_temp_dir() -> PathBuf {
        let base = std::env::temp_dir();
        let dir = base.join(format!("hlsenpai-config-test-{}", now_unix_nanos()));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    #[test]
    fn preset_round_trip_save_and_load() {
        let dir = unique_temp_dir();
        let paths = ConfigPaths::from_dir(dir);

        let mut form = EncodeOptionsForm::from_metadata(&VideoMetadata::default());
        form.gop = 48;
        form.output_base_folder = "output".to_string();
        form.variants[1].video_bitrate_k = 3333;

        let preset = PersistedEncodePreset::from_form(&form);
        save_last_preset(&paths, &preset).expect("preset should save");

        let loaded = load_last_preset(&paths)
            .expect("preset should load")
            .expect("preset should exist");

        assert_eq!(loaded, preset);
    }

    #[test]
    fn missing_files_fall_back_without_errors() {
        let dir = unique_temp_dir();
        let paths = ConfigPaths::from_dir(dir);

        let auth = load_auth_config(&paths).expect("auth config load should work");
        let preset = load_last_preset(&paths).expect("preset load should work");

        assert_eq!(auth, AppAuthConfig::default());
        assert!(preset.is_none());
    }

    #[test]
    fn corrupt_preset_file_is_backed_up_and_ignored() {
        let dir = unique_temp_dir();
        let paths = ConfigPaths::from_dir(dir.clone());

        fs::create_dir_all(&paths.dir).expect("config dir");
        fs::write(&paths.preset_file, "{not-json").expect("invalid json write");

        let loaded = load_last_preset(&paths).expect("load should not hard-fail");
        assert!(loaded.is_none());
        assert!(!paths.preset_file.exists());

        let backup_exists = fs::read_dir(dir)
            .expect("dir should be readable")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .any(|path| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().contains("broken-"))
            });
        assert!(backup_exists);
    }

    #[test]
    fn apply_preset_keeps_source_derived_fields() {
        let metadata = VideoMetadata {
            video_resolution: Some("1920x1080".to_string()),
            ..VideoMetadata::default()
        };
        let mut form = EncodeOptionsForm::from_metadata(&metadata);
        let source_width = form.source_width;
        let source_height = form.source_height;
        let source_aspect_num = form.source_aspect_num;
        let source_aspect_den = form.source_aspect_den;

        let mut preset = PersistedEncodePreset::from_form(&form);
        preset.scale_width = 854;
        preset.scale_height = 480;
        preset.gop = 42;

        apply_preset_to_form(&mut form, &preset);

        assert_eq!(form.source_width, source_width);
        assert_eq!(form.source_height, source_height);
        assert_eq!(form.source_aspect_num, source_aspect_num);
        assert_eq!(form.source_aspect_den, source_aspect_den);
        assert_eq!(form.gop, 42);
        assert_eq!(form.scale_width, 854);
        assert_eq!(form.scale_height, 480);
    }

    #[test]
    fn load_normalizes_invalid_values_and_variant_count() {
        let dir = unique_temp_dir();
        let paths = ConfigPaths::from_dir(dir);
        fs::create_dir_all(&paths.dir).expect("config dir");

        let invalid_payload = serde_json::json!({
            "version": 1,
            "updated_at_utc": null,
            "preset": {
                "scale_width": 0,
                "scale_height": 0,
                "scale_lock_aspect": false,
                "gop": 0,
                "video_codec_lib": "x264",
                "profile": "high",
                "preset": "veryfast",
                "sc_threshold": -4,
                "audio_codec": "aac",
                "audio_channels": 0,
                "hls_time_seconds": 0,
                "hls_playlist_type": "vod",
                "hls_flags_independent_segments": true,
                "master_playlist_name": "master.m3u8",
                "segment_filename_pattern": "v%v/seg_%06d.ts",
                "output_variant_playlist_pattern": "v%v/prog.m3u8",
                "output_base_folder": "out",
                "output_subfolder_name": "demo",
                "output_master_playlist_file": "master.m3u8",
                "variants": [
                    {
                        "name": "",
                        "video_bitrate_k": 0,
                        "maxrate_k": 0,
                        "bufsize_k": 0,
                        "audio_bitrate_k": 0
                    }
                ]
            }
        });
        fs::write(
            &paths.preset_file,
            serde_json::to_vec_pretty(&invalid_payload).expect("serialize test payload"),
        )
        .expect("write preset");

        let loaded = load_last_preset(&paths)
            .expect("load should succeed")
            .expect("preset should exist");

        assert_eq!(loaded.scale_width, 1);
        assert_eq!(loaded.scale_height, 1);
        assert_eq!(loaded.gop, 1);
        assert_eq!(loaded.sc_threshold, 0);
        assert_eq!(loaded.audio_channels, 1);
        assert_eq!(loaded.hls_time_seconds, 1);
        assert_eq!(loaded.variants.len(), 3);
        assert_eq!(loaded.variants[0].video_bitrate_k, 1);
        assert_eq!(loaded.variants[1].name, "med");
        assert_eq!(loaded.variants[2].name, "high");
    }

    #[test]
    fn upload_prefs_round_trip_save_and_load() {
        let dir = unique_temp_dir();
        let paths = ConfigPaths::from_dir(dir);

        let prefs = PersistedUploadPrefs {
            version: 1,
            last_upload_provider: Some("aws_s3".to_string()),
            last_overwrite_mode: Some("skip_existing".to_string()),
            last_bucket: Some("example-bucket".to_string()),
            last_prefix: Some("vod/demo".to_string()),
            updated_at_utc: None,
        };

        save_upload_prefs(&paths, &prefs).expect("upload prefs should save");
        let loaded = load_upload_prefs(&paths).expect("upload prefs should load");

        assert_eq!(loaded.last_upload_provider, Some("aws_s3".to_string()));
        assert_eq!(
            loaded.last_overwrite_mode,
            Some("skip_existing".to_string())
        );
        assert_eq!(loaded.last_bucket, Some("example-bucket".to_string()));
        assert_eq!(loaded.last_prefix, Some("vod/demo".to_string()));
    }
}
