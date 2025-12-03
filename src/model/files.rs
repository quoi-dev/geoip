use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct ArchiveFileInfo {
	pub tag: String,
	pub path: PathBuf,
	pub mtime: DateTime<Utc>,
	pub utime: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum ArchiveFileAuth {
	None,
	Bearer(String),
	Basic(String, Option<String>),
}
