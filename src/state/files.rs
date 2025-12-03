use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Weak};
use std::time::{Duration, SystemTime};
use ahash::AHashMap;
use tokio::fs;
use arc_swap::ArcSwap;
use axum::body::{Body, Bytes};
use axum::response::{IntoResponse, Response};
use axum_extra::headers::{ContentLength, ContentType, IfModifiedSince, LastModified};
use axum_extra::TypedHeader;
use chrono::{DateTime, NaiveDateTime, Utc};
use log::{error, info};
use regex::Regex;
use reqwest::{header, Client, RequestBuilder, StatusCode};
use tempfile::NamedTempFile;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::config::AppConfig;
use crate::model::{ArchiveFileAuth, ArchiveFileInfo};

const TIMESTAMP_FORMAT: &str = "%Y%m%d%H%M%S";

static ARCHIVE_NAME_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(
	"^([A-Za-z0-9-]+)-([0-9]{14})\\.tar.gz$"
).expect("Unable to compile regex"));

#[derive(Debug, Error)]
pub enum FileServiceError {
	#[error(transparent)]
	Io(#[from] io::Error),
	
	#[error(transparent)]
	Reqwest(#[from] reqwest::Error),
}

pub struct FileService {
	me: Weak<Self>,
	config: Arc<AppConfig>,
	client: Client,
	archives: ArcSwap<im::HashMap<String, Arc<ArchiveFileInfo>>>,
}

impl FileService {
	pub async fn new(config: Arc<AppConfig>, client: Client) -> Arc<Self> {
		let archives = Self::find_and_cleanup_archives(&config.data_dir).await;
		
		Arc::new_cyclic(|me| Self {
			me: me.clone(),
			config,
			client,
			archives: ArcSwap::new(archives),
		})
	}
	
	async fn find_and_cleanup_archives(
		data_dir: &Path,
	) -> Arc<im::HashMap<String, Arc<ArchiveFileInfo>>> {
		let mut out = AHashMap::new();
		if let Err(err) = fs::create_dir_all(&data_dir).await {
			error!("Unable to create data directory: {err}");
		}
		match fs::read_dir(data_dir).await {
			Ok(entries) => {
				Self::find_and_cleanup_archives_by_read_dir(data_dir, entries, &mut out).await;
			},
			Err(err) => {
				error!("Unable to read directory {}: {err}", data_dir.display());
			}
		}
		Arc::new(out.into_iter().collect())
	}
	
	async fn find_and_cleanup_archives_by_read_dir(
		path: &Path,
		mut entries: fs::ReadDir,
		out: &mut AHashMap<String, Arc<ArchiveFileInfo>>,
	) {
		loop {
			match entries.next_entry().await {
				Ok(Some(entry)) => {
					Self::find_and_cleanup_archive_by_dir_entry(entry, out).await;
				}
				Ok(None) => break,
				Err(err) => {
					error!("Unable to read {} directory entry: {err}", path.display());
					break;
				}
			}
		}
	}
	
	async fn find_and_cleanup_archive_by_dir_entry(
		entry: fs::DirEntry,
		out: &mut AHashMap<String, Arc<ArchiveFileInfo>>,
	) {
		let Some(info) = Self::archive_info_from_path(entry.path()).await else { return };
		if let Some(old) = out.insert(info.tag.clone(), info.clone()) {
			if old.mtime < info.mtime {
				Self::cleanup_archive(&old).await;
			} else {
				out.insert(info.tag.clone(), old);
				Self::cleanup_archive(&info).await;
			}
		}
	}
	
	async fn archive_info_from_path(path: PathBuf) -> Option<Arc<ArchiveFileInfo>> {
		let file_name = path.file_name()?.to_str()?;
		let captures = ARCHIVE_NAME_PATTERN.captures(file_name)?;
		let tag = captures.get(1)?.as_str();
		let mtime = captures.get(2)?.as_str();
		let mtime = NaiveDateTime::parse_from_str(mtime, TIMESTAMP_FORMAT).ok()?;
		let mtime = mtime.and_utc();
		let timestamp_path = path.with_file_name(format!("{tag}.timestamp"));
		let utime = fs::read_to_string(&timestamp_path)
			.await
			.ok()
			.and_then(|s| DateTime::parse_from_rfc2822(&s).ok())
			.map(|utime| utime.with_timezone(&Utc))
			.unwrap_or(mtime);
		Some(Arc::new(ArchiveFileInfo {
			tag: tag.to_owned(),
			path,
			mtime,
			utime,
		}))
	}
	
	pub async fn cleanup_archive(info: &ArchiveFileInfo) {
		info!("Deleting archive: {}", info.path.display());
		if let Some(data_dir) = info.path.parent() {
			if let Err(err) = Self::cleanup_data_dir(
				data_dir,
				&format!("{}-{}", info.tag, info.mtime.format(TIMESTAMP_FORMAT)),
				&info.path,
			).await {
				error!("Unable to cleanup {}: {err}", info.path.display());
			}
		}
		if let Err(err) = fs::remove_file(&info.path).await {
			error!("Unable to remove {}: {err}", info.path.display());
		}
	}
	
	async fn cleanup_data_dir(
		data_dir: &Path,
		prefix: &str,
		ignore: &Path,
	) -> Result<(), io::Error> {
		let mut entries = fs::read_dir(data_dir).await?;
		while let Some(entry) = entries.next_entry().await? {
			let path = entry.path();
			if path == ignore {
				continue;
			}
			let file_name = path.file_name().and_then(|s| s.to_str());
			let Some(file_name) = file_name else { continue };
			if !file_name.starts_with(prefix) {
				continue;
			}
			if let Err(err) = Self::remove_dir_entry(&path, &entry).await {
				error!("Unable to remove {}: {err}", entry.path().display());
			}
		}
		Ok(())
	}
	
	async fn remove_dir_entry(path: &Path, entry: &fs::DirEntry) -> Result<(), io::Error> {
		let file_type = entry.file_type().await?;
		if file_type.is_dir() {
			fs::remove_dir_all(path).await?;
		} else {
			fs::remove_file(path).await?;
		}
		Ok(())
	}
	
	pub fn get_latest_archive(&self, tag: &str) -> Option<Arc<ArchiveFileInfo>> {
		self.archives.load().get(tag).cloned()
	}
	
	pub async fn refresh_archive(
		&self,
		tag: &str,
		url: &str,
		auth: ArchiveFileAuth,
		interval: Duration,
	) -> Result<Option<Arc<ArchiveFileInfo>>, FileServiceError> {
		let now = Utc::now();
		let mut req = self.client.get(url);
		let info = self.get_latest_archive(tag);
		if let Some(info) = &info {
			if now.signed_duration_since(info.utime).as_seconds_f64() < interval.as_secs_f64() {
				info!("Skipping update of {tag}, because it is fresh enough");
				return Ok(None);
			}
			req = req.header(header::IF_MODIFIED_SINCE, httpdate::fmt_http_date(info.mtime.into()));
		}
		req = Self::setup_request_auth(req, auth);
		let mut res = req.send().await?.error_for_status()?;
		let status = res.status();
		if status == StatusCode::NOT_MODIFIED {
			self.set_refresh_timestamp(tag, now).await;
			info!("{url} wasn't modified");
			return Ok(None);
		}
		info!("Downloading {url}...");
		let mtime = res.headers()
			.get(header::LAST_MODIFIED)
			.and_then(|v| v.to_str().ok())
			.and_then(|v| httpdate::parse_http_date(v).ok())
			.map(|v| DateTime::<Utc>::from(v))
			.unwrap_or(now);
		let (file, path) = self.new_named_temp_file().await?.into_parts();
		let mut file = fs::File::from_std(file);
		while let Some(chunk) = res.chunk().await? {
			file.write_all(&chunk).await?;
		}
		let file = NamedTempFile::from_parts(file.into_std().await, path);
		let path = self.persist_named_temp_file(
			file,
			&format!("{tag}-{}.tar.gz", mtime.format(TIMESTAMP_FORMAT)),
		).await?;
		self.store_refresh_timestamp(tag, now).await?;
		info!("{tag} archive refreshed from {url}");
		let new_info = Arc::new(ArchiveFileInfo {
			tag: tag.to_owned(),
			path,
			mtime,
			utime: now,
		});
		self.archives.rcu(|archives| {
			Arc::new(archives.update(tag.to_owned(), new_info.clone()))
		});
		if let Some(info) = info {
			let cloned_info = (*info).clone();
			let info_weak = Arc::downgrade(&info);
			drop(info);
			tokio::spawn(async move {
				while info_weak.upgrade().is_some() {
					tokio::time::sleep(Duration::from_millis(100)).await;
				}
				Self::cleanup_archive(&cloned_info).await;
			});
		}
		Ok(Some(new_info))
	}
	
	fn setup_request_auth(req: RequestBuilder, auth: ArchiveFileAuth) -> RequestBuilder {
		match auth {
			ArchiveFileAuth::None => req,
			ArchiveFileAuth::Bearer(bearer) => {
				req.bearer_auth(bearer)
			}
			ArchiveFileAuth::Basic(username, password) => {
				req.basic_auth(username, password)
			}
		}
	}
	
	async fn set_refresh_timestamp(&self, tag: &str, timestamp: DateTime<Utc>) {
		if let Err(err) = self.store_refresh_timestamp(tag, timestamp).await {
			error!("Unable to store refresh timestamp for {tag}: {err}");
		}
		self.archives.rcu(|archives| {
			if let Some(info) = archives.get(tag) && info.utime < timestamp {
				let new_info = Arc::new(ArchiveFileInfo {
					utime: timestamp,
					..(**info).clone()
				});
				Arc::new(archives.update(tag.to_owned(), new_info))
			} else {
				archives.clone()
			}
		});
	}
	
	async fn store_refresh_timestamp(
		&self,
		tag: &str,
		timestamp: DateTime<Utc>,
	) -> Result<(), io::Error> {
		let (file, path) = self.new_named_temp_file().await?.into_parts();
		let mut file = fs::File::from_std(file);
		file.write_all(timestamp.to_rfc2822().as_bytes()).await?;
		let file = NamedTempFile::from_parts(file.into_std().await, path);
		self.persist_named_temp_file(file, &format!("{tag}.timestamp")).await?;
		Ok(())
	}
	
	async fn new_named_temp_file(&self) -> Result<NamedTempFile, io::Error> {
		let me = self.me.upgrade().expect("Unable to upgrade me");
		let file = tokio::task::spawn_blocking(move || {
			NamedTempFile::new_in(&me.config.data_dir)
		}).await??;
		Ok(file)
	}
	
	async fn persist_named_temp_file(
		&self,
		file: NamedTempFile,
		name: &str,
	) -> Result<PathBuf, io::Error> {
		let path = self.config.data_dir.join(name);
		let path = tokio::task::spawn_blocking(move || {
			file.persist(&path).map(|_| path)
		}).await??;
		Ok(path)
	}
	
	pub async fn download_archive(
		&self,
		info: Arc<ArchiveFileInfo>,
		file_name: &str,
		if_modified_since: Option<TypedHeader<IfModifiedSince>>,
	) -> Result<Response, FileServiceError> {
		if let Some(if_modified_since) = if_modified_since {
			if !if_modified_since.is_modified(info.mtime.into()) {
				return Ok((
					StatusCode::NOT_MODIFIED,
					TypedHeader(LastModified::from(SystemTime::from(info.mtime))),
				).into_response());
			}
		}
		let file = fs::File::open(&info.path).await?;
		let metadata = file.metadata().await?;
		let len = metadata.len();
		let stream = Body::from_stream(futures::stream::unfold(
			Some((file, info.clone())),
			|state| async move {
				let (mut file, handle) = state?;
				let mut buf = [0u8; 8192];
				match file.read(&mut buf).await {
					Ok(0) => None,
					Ok(count) => {
						let bytes = Bytes::copy_from_slice(&buf[..count]);
						Some((Ok(bytes), Some((file, handle))))
					},
					Err(err) => Some((Err(err), None))
				}
			}
		));
		Ok((
			StatusCode::OK,
			[(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{file_name}\""))],
			TypedHeader(LastModified::from(SystemTime::from(info.mtime))),
			TypedHeader(ContentType::octet_stream()),
			TypedHeader(ContentLength(len)),
			stream,
		).into_response())
	}
}
