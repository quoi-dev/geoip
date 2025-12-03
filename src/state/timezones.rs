use std::fs::{DirEntry, File};
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::{Arc, Weak};
use std::time::Duration;
use ahash::AHashMap;
use arc_swap::ArcSwapOption;
use flate2::read::GzDecoder;
use log::{error, info};
use tar::Archive;
use thiserror::Error;
use tokio::process::Command;
use tokio::time::MissedTickBehavior;
use crate::config::AppConfig;
use crate::model::{ArchiveFileAuth, ArchiveFileInfo};
use crate::state::{FileService, FileServiceError};

#[derive(Debug, Error)]
pub enum TimezoneServiceError {
	#[error(transparent)]
	Io(#[from] std::io::Error),
	
	#[error(transparent)]
	FileService(#[from] FileServiceError),
	
	#[error(transparent)]
	Join(#[from] tokio::task::JoinError),
	
	#[error("Invalid file name")]
	InvalidFileName,
	
	#[error("ZIC command failed with exit code {0}")]
	Zic(ExitStatus)
}

pub struct TimezoneService {
	me: Weak<Self>,
	config: Arc<AppConfig>,
	files: Arc<FileService>,
	zic_path: Option<PathBuf>,
	timezones: ArcSwapOption<AHashMap<String, String>>,
}

impl TimezoneService {
	pub async fn new(
		config: Arc<AppConfig>,
		files: Arc<FileService>,
	) -> Arc<Self> {
		let zic_path = Self::find_zic(&config);
		let mut timezones = Self::load_from_system();
		if let Some(zic_path) = &zic_path {
			if let Some(info) = files.get_latest_archive("tzdata") {
				match Self::load_from_archive(&info, zic_path).await {
					Ok(tz) => timezones = tz,
					Err(err) => {
						error!("Unable to load timezone database from archive: {err}");
					}
				}
			}
		}
		
		Arc::new_cyclic(|me| Self {
			me: me.clone(),
			config,
			files,
			zic_path,
			timezones: ArcSwapOption::from_pointee(timezones),
		})
	}
	
	fn find_zic(config: &AppConfig) -> Option<PathBuf> {
		if let Some(zic_path) = &config.zic_path {
			info!("zic path: {zic_path}");
			return Some(PathBuf::from(zic_path));
		}
		match which::which("zic") {
			Ok(zic_path) => {
				info!("zic path: {}", zic_path.display());
				Some(zic_path)
			}
			Err(err) => {
				error!("Unable to find zic path: {err}");
				None
			}
		}
	}
	
	async fn load_from_archive(
		info: &ArchiveFileInfo,
		zic_path: &Path,
	) -> Result<AHashMap<String, String>, TimezoneServiceError> {
		let path = info.path.clone();
		let path = tokio::task::spawn_blocking(move || Self::decompress(&path)).await??;
		let path = Self::compile(&path, zic_path).await?;
		let mut timezones = AHashMap::new();
		Self::load_from_dir(&path, "", &mut timezones)?;
		info!("Loaded {} timezones from {}", timezones.len(), info.path.display());
		Ok(timezones)
	}
	
	fn load_from_system() -> AHashMap<String, String> {
		let mut out = AHashMap::new();
		let path = Path::new("/usr/share/zoneinfo");
		Self::maybe_load_from_dir(path, "", &mut out);
		info!("Loaded {} system timezones", out.len());
		out
	}
	
	fn maybe_load_from_dir(
		path: &Path,
		prefix: &str,
		out: &mut AHashMap<String, String>,
	) {
		if let Err(err) = Self::load_from_dir(path, prefix, out) {
			error!("Unable to load timezones from {}: {err}", path.display());
		}
	}
	
	fn load_from_dir(
		path: &Path,
		prefix: &str,
		out: &mut AHashMap<String, String>,
	) -> Result<(), TimezoneServiceError> {
		for entry in std::fs::read_dir(path)? {
			let entry = entry?;
			Self::maybe_load_from_dir_entry(&entry, prefix, out);
		}
		Ok(())
	}
	
	fn maybe_load_from_dir_entry(
		entry: &DirEntry,
		prefix: &str,
		out: &mut AHashMap<String, String>,
	) {
		if let Err(err) = Self::load_from_dir_entry(entry, prefix, out) {
			error!("Unable to load timezones from {}: {err}", entry.path().display());
		}
	}
	
	fn load_from_dir_entry(
		entry: &DirEntry,
		prefix: &str,
		out: &mut AHashMap<String, String>,
	) -> Result<(), TimezoneServiceError> {
		let file_name = entry.file_name();
		let file_name = file_name.to_str()
			.ok_or_else(|| TimezoneServiceError::InvalidFileName)?;
		let file_type = entry.file_type()?;
		if file_type.is_dir() {
			Self::load_from_dir(&entry.path(), &format!("{prefix}{file_name}/"), out)?;
		} else if file_type.is_file() {
			let bytes = std::fs::read(entry.path())?;
			Self::load_from_slice(&format!("{prefix}{file_name}"), &bytes, out)?;
		}
		Ok(())
	}
	
	fn load_from_slice(
		name: &str,
		bytes: &[u8],
		out: &mut AHashMap<String, String>,
	) -> Result<(), TimezoneServiceError> {
		if !bytes.starts_with(b"TZif") {
			return Ok(());
		}
		let Some(bytes) = bytes.strip_suffix(b"\n") else { return Ok(()) };
		let Some(i) = bytes.iter().rposition(|c| *c == b'\n') else { return Ok(()) };
		let spec = &bytes[i + 1..];
		let Ok(spec) = str::from_utf8(spec) else { return Ok(()) };
		let spec = spec.trim();
		if spec.is_empty() {
			return Ok(());
		}
		out.insert(name.to_owned(), spec.to_owned());
		Ok(())
	}
	
	pub fn start_updater(&self) {
		let me = self.me.upgrade().expect("Unable to upgrade me");
		if self.zic_path.is_none() {
			info!("Cannot perform timezone database auto-update without zic executable");
			return;
		}
		let Some(interval) = self.config.tzdata_auto_update_interval else {
			info!("Timezone database auto-update is disabled");
			return;
		};
		info!("Timezone database auto-update interval: {interval} hour(s)");
		tokio::task::spawn(async move {
			let mut interval = tokio::time::interval(Duration::from_hours(interval));
			interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
			loop {
				interval.tick().await;
				if let Err(err) = me.update().await {
					error!("Unable to update timezone database: {err}");
				}
			}
		});
	}
	
	async fn update(&self) -> Result<(), TimezoneServiceError> {
		let Some(zic_path) = self.zic_path.as_ref() else { return Ok(()) };
		info!("Updating timezone database...");
		let info = self.files.refresh_archive(
			"tzdata",
			&self.config.tzdata_download_url,
			if let Some(token) = &self.config.tzdata_bearer_token {
				ArchiveFileAuth::Bearer(token.clone())
			} else {
				ArchiveFileAuth::None
			},
			Duration::from_hours(self.config.tzdata_auto_update_interval.unwrap_or(0)),
		).await?;
		let Some(info) = info else { return Ok(()) };
		let timezones = Self::load_from_archive(&info, zic_path).await?;
		self.timezones.store(Some(Arc::new(timezones)));
		info!("Using new timezone database");
		Ok(())
	}
	
	fn decompress(path: &Path) -> Result<PathBuf, TimezoneServiceError> {
		let mut out_dir = path.with_extension("");
		out_dir.set_extension("");
		if out_dir.exists() {
			return Ok(out_dir);
		}
		let file = File::open(path)?;
		let decoder = GzDecoder::new(file);
		let mut archive = Archive::new(decoder);
		archive.unpack(&out_dir)?;
		Ok(out_dir)
	}
	
	async fn compile(path: &Path, zic_path: &Path) -> Result<PathBuf, TimezoneServiceError> {
		let out_dir = path.with_added_extension("zoneinfo");
		let abs_out_dir = std::path::absolute(&out_dir)?;
		tokio::fs::create_dir_all(&out_dir).await?;
		let mut cmd = Command::new(zic_path);
		cmd.current_dir(path);
		cmd.arg("-d").arg(abs_out_dir);
		cmd.arg("-L").arg("leapseconds");
		const TZ_FILES: &[&str] = &[
			"africa",
			"antarctica",
			"asia",
			"australasia",
			"etcetera",
			"europe",
			"northamerica",
			"southamerica",
			"backward",
		];
		for tz_file in TZ_FILES {
			cmd.arg(tz_file);
		}
		let res = cmd.output().await?;
		for line in res.stderr.split(|c| *c == b'\n') {
			if line.is_empty() {
				continue;
			}
			error!("ZIC output: {}", String::from_utf8_lossy(line));
		}
		if !res.status.success() {
			return Err(TimezoneServiceError::Zic(res.status));
		}
		Ok(out_dir)
	}
	
	pub fn get_all(&self) -> AHashMap<String, String> {
		self.timezones.load()
			.as_ref()
			.map(|zones| (**zones).clone())
			.unwrap_or_default()
	}
	
	pub fn lookup(&self, id: &str) -> Option<String> {
		self.timezones.load()
			.as_ref()
			.and_then(|zones| zones.get(id).cloned())
	}
	
	pub fn get_archive(&self) -> Option<Arc<ArchiveFileInfo>> {
		self.files.get_latest_archive("tzdata")
	}
}
