use std::fs;
use std::fs::DirEntry;
use std::path::Path;
use std::sync::Arc;
use ahash::AHashMap;
use arc_swap::ArcSwapOption;
use log::error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TimezoneServiceError {
	#[error(transparent)]
	Io(#[from] std::io::Error),
	
	#[error("Invalid file name")]
	InvalidFileName,
}

pub struct TimezoneService {
	timezones: ArcSwapOption<AHashMap<String, String>>,
}

impl TimezoneService {
	pub fn new() -> Arc<Self> {
		let timezones = Self::load_from_system();
		
		Arc::new(Self {
			timezones: ArcSwapOption::from_pointee(timezones),
		})
	}
	
	fn load_from_system() -> AHashMap<String, String> {
		let mut out = AHashMap::new();
		let path = Path::new("/usr/share/zoneinfo");
		Self::maybe_load_from_dir(path, "", &mut out);
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
		for entry in fs::read_dir(path)? {
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
			let bytes = fs::read(entry.path())?;
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
}
