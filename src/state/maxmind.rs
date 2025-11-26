use std::{fs, io};
use std::ffi::OsStr;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Weak};
use std::time::Duration;
use ahash::AHashMap;
use arc_swap::ArcSwapOption;
use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};
use flate2::read::GzDecoder;
use log::{error, info, warn};
use maxminddb::{geoip2, MaxMindDbError};
use regex::Regex;
use reqwest::{header, Client, StatusCode};
use tempfile::NamedTempFile;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::time::MissedTickBehavior;
use crate::config::{AppConfig, DOWNLOAD_URL_EDITION_PLACEHOLDER};
use crate::model::{GeoIpDatabaseStatus, GeoIpInfo, GeoIpStatus, GeoNameSubdivision};

#[derive(Debug, Error)]
pub enum MaxMindServiceError {
	#[error(transparent)]
	Io(#[from] io::Error),
	
	#[error(transparent)]
	MaxMindDb(#[from] MaxMindDbError),
	
	#[error(transparent)]
	Reqwest(#[from] reqwest::Error),
	
	#[error(transparent)]
	JoinError(#[from] tokio::task::JoinError),
	
	#[error("Unknown MaxMind database edition")]
	UnknownEdition,
	
	#[error("MaxMind database is missing")]
	MissingDatabase,
	
	#[error("HTTP error (status={0})")]
	HttpError(StatusCode),
}

struct MaxMindDbReader {
	path: PathBuf,
	reader: maxminddb::Reader<maxminddb::Mmap>,
	file_size: u64,
	archive_file_size: Option<u64>,
}

pub struct MaxMindService {
	me: Weak<Self>,
	config: Arc<AppConfig>,
	client: Client,
	readers: AHashMap<String, ArcSwapOption<MaxMindDbReader>>,
	errors: AHashMap<String, ArcSwapOption<String>>,
}

static MMDB_FILENAME_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(
	"^([A-Za-z0-9-]+)-([0-9]{14}).mmdb$"
).expect("Unable to compile regex"));

impl MaxMindService {
	pub fn new(config: Arc<AppConfig>, client: Client) -> Arc<Self> {
		let (
			readers,
			errors,
		) = Self::load_all_latest(&config);
		
		Arc::new_cyclic(|me| Self {
			me: me.clone(),
			config,
			client,
			readers,
			errors,
		})
	}
	
	fn load_all_latest(config: &AppConfig) -> (
		AHashMap<String, ArcSwapOption<MaxMindDbReader>>,
		AHashMap<String, ArcSwapOption<String>>,
	) {
		let databases = Self::enumerate_databases(&config);
		let mut out = AHashMap::new();
		let mut errors = AHashMap::new();
		for edition in &config.maxmind_editions {
			errors.insert(edition.clone(), ArcSwapOption::new(None));
			let out_err = errors.get(edition).expect("Unknown edition");
			let mut reader = None;
			if let Some(versions) = databases.get(edition) {
				for (_, path) in versions.iter().rev() {
					if reader.is_some() {
						Self::cleanup(path);
						continue;
					}
					match Self::load(path) {
						Ok(r) => {
							reader = Some(r);
							out_err.store(None);
						}
						Err(err) => {
							warn!("Unable to open MaxMind {edition} database: {err}");
							out_err.store(Some(Arc::new(err.to_string())));
							Self::cleanup(path);
						}
					}
				}
			}
			if reader.is_none() {
				warn!("No available versions for {edition} MaxMind database");
			}
			out.insert(edition.clone(), ArcSwapOption::from(reader));
		}
		(out, errors)
	}
	
	fn load(path: &Path) -> Result<Arc<MaxMindDbReader>, MaxMindServiceError> {
		info!("Opening MaxMind database {:?}", path);
		let file_size = fs::metadata(path)?.len();
		let archive_file_size = fs::metadata(path.with_extension("tar.gz"))
			.ok()
			.map(|metadata| metadata.len());
		let reader = maxminddb::Reader::open_mmap(&path)?;
		info!(
			"Opened MaxMind database (type={}, build_epoch={})",
			reader.metadata.database_type,
			reader.metadata.build_epoch,
		);
		Ok(Arc::new(MaxMindDbReader {
			path: path.to_path_buf(),
			reader,
			file_size,
			archive_file_size,
		}))
	}
	
	fn enumerate_databases(config: &AppConfig) -> AHashMap<String, Vec<(DateTime<Utc>, PathBuf)>> {
		let mut out = AHashMap::new();
		let Ok(entries) = fs::read_dir(&config.data_dir) else { return out };
		for entry in entries {
			let entry = match entry {
				Ok(entry) => entry,
				Err(err) => {
					error!("Unable to read {}: {err}", config.data_dir.display());
					continue;
				}
			};
			let path = entry.path();
			let Some((
				edition,
				mtime
			)) = Self::parse_edition_and_mtime_from_path(&path) else { continue };
			let versions = out
				.entry(edition.to_string())
				.or_insert_with(Vec::new);
			versions.push((mtime, path));
		}
		for versions in out.values_mut() {
			versions.sort_unstable_by_key(|(mtime, _)| *mtime);
		}
		out
	}
	
	pub fn start_updater(&self) {
		let me = self.me.upgrade().expect("Unable to upgrade me");
		if !self.config.auto_update {
			info!("Auto-update is disabled");
			return;
		} else {
			info!("Auto-update interval: {} hour(s)", self.config.auto_update_interval);
		}
		tokio::task::spawn(async move {
			info!("Started background auto-updater");
			let mut interval = tokio::time::interval(
				Duration::from_hours(me.config.auto_update_interval),
			);
			interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
			loop {
				interval.tick().await;
				me.update_all().await;
			}
		});
	}
	
	async fn update_all(&self) {
		info!("Updating all databases");
		let now = Utc::now();
		let min_time_delta = TimeDelta::hours(self.config.auto_update_interval as i64);
		for edition in &self.config.maxmind_editions {
			if let Some(mtime) = self.get_edition_mtime(edition) {
				if now.signed_duration_since(mtime) < min_time_delta {
					info!(
						"Skipping update for {edition}, because it is newer than {} hour(s)",
						self.config.auto_update_interval,
					);
					continue;
				}
			}
			let res = self.update(edition).await;
			if let Some(out_err) = self.errors.get(edition) {
				out_err.store(res.err().map(|err| Arc::new(err.to_string())));
			}
		}
	}
	
	async fn update(&self, edition: &str) -> Result<(), MaxMindServiceError> {
		info!("Updating {edition}...");
		let out_reader = self.readers
			.get(edition)
			.ok_or(MaxMindServiceError::UnknownEdition)?;
		let path = self.download(edition).await?;
		let Some(path) = path else { return Ok(()) };
		let file_size = fs::metadata(&path)?.len();
		let archive_file_size = fs::metadata(path.with_extension("tar.gz"))
			.ok()
			.map(|metadata| metadata.len());
		let reader = match maxminddb::Reader::open_mmap(&path) {
			Ok(reader) => reader,
			Err(err) => {
				error!("Unable to open {}: {err}", path.display());
				if let Err(err) = tokio::fs::remove_file(&path).await {
					error!("Unable to remove corrupted MaxMind database {}: {err}", path.display());
				}
				return Err(err.into());
			}
		};
		let old = out_reader.swap(Some(Arc::new(MaxMindDbReader {
			path: path.clone(),
			reader,
			file_size,
			archive_file_size,
		})));
		info!("Using {}", path.display());
		Self::wait_unused_and_cleanup(old).await;
		Ok(())
	}
	
	async fn wait_unused_and_cleanup(reader: Option<Arc<MaxMindDbReader>>) {
		let Some(reader) = reader else { return };
		let path = reader.path.clone();
		let weak = Arc::downgrade(&reader);
		drop(reader);
		while weak.upgrade().is_some() {
			tokio::time::sleep(Duration::from_millis(100)).await;
		}
		Self::cleanup(&path);
	}
	
	fn cleanup(path: &Path) {
		if let Err(err) = fs::remove_file(&path) {
			error!("Unable to delete outdated MaxMind database {}: {err}", path.display());
		} else {
			info!("Deleted outdated MaxMind database {}", path.display());
		}
		let archive_path = path.with_added_extension("tar.gz");
		if let Err(err) = fs::remove_file(&archive_path) {
			error!("Unable to remove outdated MaxMind database archive {}: {err}", archive_path.display());
		} else {
			info!("Removed outdated MaxMind database archive {}", archive_path.display());
		}
	}
	
	async fn download(
		&self,
		edition: &str,
	) -> Result<Option<PathBuf>, MaxMindServiceError> {
		let mtime = self.get_edition_mtime(edition);
		let url = self.make_download_url(edition);
		tokio::fs::create_dir_all(&self.config.data_dir).await?;
		let mut req = self.client.get(&url);
		if let Some(username) = &self.config.maxmind_account_id {
			info!("Using HTTP basic auth");
			req = req.basic_auth(username, self.config.maxmind_license_key.as_ref());
		}
		if let Some(mtime) = mtime {
			let mtime_str = mtime.to_rfc2822();
			info!("Using If-Modified-Since: {mtime_str}");
			req = req.header(header::IF_MODIFIED_SINCE, mtime_str);
		}
		info!("Downloading {url}...");
		let mut res = req.send().await?;
		let status = res.status();
		if status == StatusCode::NOT_MODIFIED {
			if let Some(mtime) = mtime {
				info!("{url} wasn't modified since {mtime}");
			} else {
				info!("{url} wasn't modified");
			}
			return Ok(None);
		}
		if !status.is_success() {
			info!("Unable to retrieve {url} with HTTP status {status}");
			return Err(MaxMindServiceError::HttpError(status));
		}
		let last_modified = res.headers()
			.get(header::LAST_MODIFIED)
			.and_then(|s| s.to_str().ok())
			.and_then(|s| DateTime::parse_from_rfc2822(s).ok())
			.map(|t| t.to_utc())
			.unwrap_or_else(|| Utc::now());
		info!("Last modified: {}", last_modified.to_rfc2822());
		let file = NamedTempFile::new_in(&self.config.data_dir)?;
		let (file, tmp_path) = file.into_parts();
		let mut file = tokio::fs::File::from_std(file);
		let mut received_length: u64 = 0;
		while let Some(chunk) = res.chunk().await? {
			file.write_all(&chunk).await?;
			received_length += chunk.len() as u64;
		}
		info!("Downloaded {received_length} bytes from {url}");
		let file = NamedTempFile::from_parts(file.into_std().await, tmp_path);
		let archive_path = self.make_archive_path(edition, last_modified);
		let out_path = self.make_mmdb_path(edition, last_modified);
		file.persist(&archive_path).map_err(|err| err.error)?;
		info!("Downloaded {url} into {}", archive_path.display());
		let cloned_out_path = out_path.clone();
		let cloned_archive_path = archive_path.clone();
		let res = tokio::task::spawn_blocking(move || {
			Self::extract_mmdb(&cloned_archive_path, &cloned_out_path)
		}).await??;
		if !res {
			return Ok(None);
		}
		Ok(Some(out_path))
	}
	
	fn extract_mmdb(
		archive_path: &Path,
		out_path: &Path,
	) -> Result<bool, MaxMindServiceError> {
		let mut file = fs::File::open(archive_path)?;
		let gz = GzDecoder::new(&mut file);
		let mut archive = tar::Archive::new(gz);
		for entry in archive.entries()? {
			let mut entry = entry?;
			let path = entry.path()?;
			if path.extension().and_then(OsStr::to_str) == Some("mmdb") {
				info!("Extracting {}...", path.display());
				entry.unpack(out_path)?;
				return Ok(true);
			}
		}
		Ok(false)
	}
	
	fn get_edition_mtime(&self, edition: &str) -> Option<DateTime<Utc>> {
		let reader = self.readers.get(edition)?;
		let reader = reader.load_full()?;
		let (_, mtime) = Self::parse_edition_and_mtime_from_path(&reader.path)?;
		Some(mtime)
	}
	
	fn parse_edition_and_mtime_from_path(path: &Path) -> Option<(&str, DateTime<Utc>)> {
		let filename = path.file_name()?.to_str()?;
		let captures = MMDB_FILENAME_PATTERN.captures(filename)?;
		let edition = captures.get(1)?.as_str();
		let mtime = &captures[2];
		let mtime = NaiveDateTime::parse_from_str(mtime, "%Y%m%d%H%M%S").ok()?;
		Some((edition, mtime.and_utc()))
	}
	
	fn make_download_url(&self, edition: &str) -> String {
		self.config.maxmind_download_url.replace(
			DOWNLOAD_URL_EDITION_PLACEHOLDER,
			edition,
		)
	}
	
	fn make_archive_path(&self, edition: &str, last_modified: DateTime<Utc>) -> PathBuf {
		let timestamp = last_modified.format("%Y%m%d%H%M%S").to_string();
		self.config.data_dir.join(format!("{edition}-{timestamp}.tar.gz"))
	}
	
	fn make_mmdb_path(&self, edition: &str, last_modified: DateTime<Utc>) -> PathBuf {
		let timestamp = last_modified.format("%Y%m%d%H%M%S").to_string();
		self.config.data_dir.join(format!("{edition}-{timestamp}.mmdb"))
	}
	
	pub fn status(&self) -> GeoIpStatus {
		let databases = self.config.maxmind_editions
			.iter()
			.map(|edition| self.get_edition_status(edition))
			.collect();
		GeoIpStatus {
			databases,
		}
	}
	
	fn get_edition_status(&self, edition: &str) -> GeoIpDatabaseStatus {
		let mut status = GeoIpDatabaseStatus {
			edition: edition.to_owned(),
			timestamp: None,
			file_size: None,
			archive_file_size: None,
			error: None,
		};
		if let Some(reader) = self.readers.get(edition).map(ArcSwapOption::load) {
			if let Some(reader) = reader.as_ref() {
				status.timestamp = DateTime::from_timestamp_secs(
					reader.reader.metadata.build_epoch as i64,
				);
				status.file_size = Some(reader.file_size);
				status.archive_file_size = reader.archive_file_size;
			}
		}
		status.error = self.errors.get(edition).and_then(|err| {
			err.load().as_ref().map(|err| (**err).clone())
		});
		status
	}
	
	pub fn lookup(
		&self,
		ip: IpAddr,
		locale: &str,
		edition: Option<&str>,
	) -> Result<Option<GeoIpInfo>, MaxMindServiceError> {
		let reader = self.get_reader(edition)?;
		if reader.reader.metadata.database_type == "GeoLite2-ASN" {
			let res = reader.reader.lookup::<geoip2::Asn>(ip)?;
			let Some(res) = res else { return Ok(None) };
			return Ok(Some(GeoIpInfo {
				autonomous_system_number: res.autonomous_system_number,
				autonomous_system_organization: res.autonomous_system_organization.map(str::to_owned),
				..Default::default()
			}));
		}
		let res = reader.reader.lookup::<geoip2::Enterprise>(ip)?;
		let Some(res) = res else { return Ok(None) };
		Ok(Some(GeoIpInfo {
			continent_id: res.continent.as_ref().and_then(|c| c.geoname_id),
			continent_code: res.continent.as_ref().and_then(|c| c.code).map(str::to_owned),
			continent_name: res.continent
				.as_ref()
				.and_then(|c| c.names.as_ref())
				.and_then(|c| c.get(locale))
				.map(|c| (*c).to_owned()),
			country_id: res.country.as_ref().and_then(|c| c.geoname_id),
			country_iso_code: res.country.as_ref().and_then(|c| c.iso_code).map(str::to_owned),
			country_name: res.country.as_ref()
				.and_then(|c| c.names.as_ref())
				.and_then(|c| c.get(locale))
				.map(|c| (*c).to_owned()),
			subdivisions: res.subdivisions.iter().flatten().map(|s| GeoNameSubdivision {
				id: s.geoname_id,
				iso_code: s.iso_code.map(str::to_owned),
				name: s.names.as_ref()
					.and_then(|n| n.get(locale))
					.map(|n| (*n).to_owned()),
			}).collect(),
			city_id: res.city.as_ref().and_then(|c| c.geoname_id),
			city_name: res.city.as_ref()
				.and_then(|c| c.names.as_ref())
				.and_then(|c| c.get(locale))
				.map(|c| (*c).to_owned()),
			metro_code: res.location.as_ref().and_then(|c| c.metro_code),
			postal_code: res.postal.as_ref().and_then(|c| c.code).map(str::to_owned),
			timezone: res.location.as_ref().and_then(|c| c.time_zone).map(str::to_owned),
			latitude: res.location.as_ref().and_then(|c| c.latitude),
			longitude: res.location.as_ref().and_then(|c| c.longitude),
			accuracy_radius: res.location.as_ref().and_then(|c| c.accuracy_radius),
			is_in_european_union: res.country.as_ref().and_then(|c| c.is_in_european_union),
			is_anonymous_proxy: res.traits.as_ref().and_then(|c| c.is_anonymous_proxy),
			is_anycast: res.traits.as_ref().and_then(|c| c.is_anycast),
			is_satellite_provider: res.traits.as_ref().and_then(|c| c.is_satellite_provider),
			autonomous_system_number: res.traits.as_ref().and_then(|c| c.autonomous_system_number),
			autonomous_system_organization: res.traits.as_ref()
				.and_then(|c| c.autonomous_system_organization)
				.map(str::to_owned),
		}))
	}
	
	fn get_reader(
		&self,
		edition: Option<&str>,
	) -> Result<Arc<MaxMindDbReader>, MaxMindServiceError> {
		let reader = edition
			.or_else(|| self.default_edition())
			.and_then(|edition| self.readers.get(edition))
			.ok_or(MaxMindServiceError::UnknownEdition)?
			.load_full()
			.ok_or(MaxMindServiceError::MissingDatabase)?;
		Ok(reader)
	}
	
	pub fn default_edition(&self) -> Option<&str> {
		self.config.maxmind_editions.first().map(String::as_str)
	}
}
