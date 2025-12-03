use std::{fs, io};
use std::ffi::OsStr;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use std::time::Duration;
use ahash::AHashMap;
use arc_swap::ArcSwapOption;
use chrono::DateTime;
use flate2::read::GzDecoder;
use log::{error, info, warn};
use maxminddb::{geoip2, MaxMindDbError};
use thiserror::Error;
use tokio::time::MissedTickBehavior;
use crate::config::{AppConfig, DOWNLOAD_URL_EDITION_PLACEHOLDER};
use crate::model::{ArchiveFileAuth, ArchiveFileInfo, GeoIpDatabaseStatus, GeoIpInfo, GeoIpStatus, GeoNameSubdivision};
use crate::state::{FileService, FileServiceError, TimezoneService};

#[derive(Debug, Error)]
pub enum MaxMindServiceError {
	#[error(transparent)]
	Io(#[from] io::Error),
	
	#[error(transparent)]
	MaxMindDb(#[from] MaxMindDbError),
	
	#[error(transparent)]
	FileService(#[from] FileServiceError),
	
	#[error(transparent)]
	JoinError(#[from] tokio::task::JoinError),
	
	#[error("Unknown MaxMind database edition")]
	UnknownEdition,
	
	#[error("MaxMind database is missing")]
	MissingDatabase,
}

struct MaxMindDbReader {
	path: PathBuf,
	reader: maxminddb::Reader<maxminddb::Mmap>,
	file_size: u64,
	archive_file_size: Option<u64>,
	info: Arc<ArchiveFileInfo>,
}

pub struct MaxMindService {
	me: Weak<Self>,
	config: Arc<AppConfig>,
	files: Arc<FileService>,
	timezones: Arc<TimezoneService>,
	readers: AHashMap<String, ArcSwapOption<MaxMindDbReader>>,
	errors: AHashMap<String, ArcSwapOption<String>>,
}

impl MaxMindService {
	pub async fn new(
		config: Arc<AppConfig>,
		files: Arc<FileService>,
		timezones: Arc<TimezoneService>,
	) -> Arc<Self> {
		let (
			readers,
			errors,
		) = Self::load_all_latest(&config, &files).await;
		
		Arc::new_cyclic(|me| Self {
			me: me.clone(),
			config,
			files,
			timezones,
			readers,
			errors,
		})
	}
	
	async fn load_all_latest(config: &AppConfig, files: &FileService) -> (
		AHashMap<String, ArcSwapOption<MaxMindDbReader>>,
		AHashMap<String, ArcSwapOption<String>>,
	) {
		let mut out = AHashMap::new();
		let mut errors = AHashMap::new();
		for edition in &config.maxmind_editions {
			errors.insert(edition.clone(), ArcSwapOption::new(None));
			let out_err = errors.get(edition).expect("Unknown edition");
			let mut reader = None;
			if let Some(info) = files.get_latest_archive(edition) {
				match Self::load_from_archive(info.clone()) {
					Ok(r) => {
						reader = Some(r);
						out_err.store(None);
					}
					Err(err) => {
						warn!("Unable to open MaxMind {edition} database: {err}");
						out_err.store(Some(Arc::new(err.to_string())));
						FileService::cleanup_archive(&info).await;
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
	
	fn load_from_archive(
		info: Arc<ArchiveFileInfo>,
	) -> Result<Arc<MaxMindDbReader>, MaxMindServiceError> {
		let mut path = info.path.with_extension("");
		path.set_extension("mmdb");
		if !path.exists() {
			if !Self::extract_mmdb(&info.path, &path)? {
				return Err(MaxMindServiceError::MissingDatabase);
			}
		}
		let file_size = path.metadata()?.len();
		let archive_file_size = info.path.metadata()?.len();
		let reader = maxminddb::Reader::open_mmap(&path)?;
		info!(
			"Opened MaxMind database (type={}, build_epoch={})",
			reader.metadata.database_type,
			reader.metadata.build_epoch,
		);
		Ok(Arc::new(MaxMindDbReader {
			path,
			reader,
			file_size,
			archive_file_size: Some(archive_file_size),
			info,
		}))
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
		for edition in &self.config.maxmind_editions {
			let res = self.update(edition).await;
			if let Some(out_err) = self.errors.get(edition) {
				out_err.store(res.err().map(|err| Arc::new(err.to_string())));
			}
		}
	}
	
	async fn update(&self, edition: &str) -> Result<(), MaxMindServiceError> {
		info!("Updating {edition}...");
		let url = self.make_download_url(edition);
		let auth = self.make_archive_auth();
		let info = self.files.refresh_archive(
			edition,
			&url,
			auth,
			Duration::from_hours(self.config.auto_update_interval),
		).await?;
		let Some(info) = info else { return Ok(()) };
		let reader = Self::load_from_archive(info.clone())?;
		let out_reader = self.readers
			.get(edition)
			.ok_or(MaxMindServiceError::UnknownEdition)?;
		out_reader.store(Some(reader.clone()));
		info!("Using {}", reader.path.display());
		Ok(())
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
	
	fn make_download_url(&self, edition: &str) -> String {
		self.config.maxmind_download_url.replace(
			DOWNLOAD_URL_EDITION_PLACEHOLDER,
			edition,
		)
	}
	
	fn make_archive_auth(&self) -> ArchiveFileAuth {
		if let Some(token) = &self.config.maxmind_bearer_token {
			ArchiveFileAuth::Bearer(token.clone())
		} else if let Some(username) = &self.config.maxmind_account_id {
			ArchiveFileAuth::Basic(
				username.clone(),
				self.config.maxmind_license_key.clone(),
			)
		} else {
			ArchiveFileAuth::None
		}
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
			locales: Vec::new(),
			file_size: None,
			archive_file_size: None,
			error: None,
			last_update_check: None,
		};
		if let Some(reader) = self.readers.get(edition).map(ArcSwapOption::load) {
			if let Some(reader) = reader.as_ref() {
				status.timestamp = DateTime::from_timestamp_secs(
					reader.reader.metadata.build_epoch as i64,
				);
				status.locales = reader.reader.metadata.languages.clone();
				status.file_size = Some(reader.file_size);
				status.archive_file_size = reader.archive_file_size;
				status.last_update_check = Some(reader.info.utime);
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
			posix_timezone: res.location.as_ref()
				.and_then(|c| c.time_zone)
				.and_then(|zone| self.timezones.lookup(zone)),
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
	
	pub fn get_archive(&self, edition: &str) -> Result<Arc<ArchiveFileInfo>, MaxMindServiceError> {
		Ok(self.get_reader(Some(edition))?.info.clone())
	}
	
	pub fn default_edition(&self) -> Option<&str> {
		self.config.maxmind_editions.first().map(String::as_str)
	}
}
