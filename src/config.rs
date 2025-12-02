use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

const DEFAULT_EDITIONS: &str = "GeoLite2-City";
pub const DOWNLOAD_URL_EDITION_PLACEHOLDER: &str = "{edition}";
const DOWNLOAD_URL: &str = "https://download.maxmind.com/geoip/databases/{edition}/download?suffix=tar.gz";

pub struct AppConfig {
	pub listen_addr: SocketAddr,
	pub data_dir: PathBuf,
	pub maxmind_account_id: Option<String>,
	pub maxmind_license_key: Option<String>,
	pub maxmind_bearer_token: Option<String>,
	pub maxmind_editions: Vec<String>,
	pub maxmind_download_url: String,
	pub auto_update: bool,
	pub auto_update_interval: u64,
	pub api_key: Option<String>,
	pub recaptcha_site_key: Option<String>,
	pub recaptcha_secret_key: Option<String>,
}

impl AppConfig {
	pub fn load_from_env() -> Arc<Self> {
		let listen_addr = env::var("LISTEN_ADDR").ok()
			.unwrap_or_else(|| "127.0.0.1:8080".to_owned())
			.parse()
			.expect("LISTEN_ADDR must be a valid socket address");
		let data_dir = env::var("DATA_DIR").ok()
			.map(PathBuf::from)
			.expect("DATA_DIR must be set");
		let maxmind_account_id = env::var("MAXMIND_ACCOUNT_ID").ok();
		let maxmind_license_key = env::var("MAXMIND_LICENCE_KEY").ok();
		let maxmind_bearer_token = env::var("MAXMIND_BEARER_TOKEN").ok();
		let maxmind_editions = env::var("MAXMIND_EDITIONS").ok()
			.unwrap_or_else(|| DEFAULT_EDITIONS.to_owned())
			.split(',')
			.map(str::trim)
			.map(str::to_owned)
			.collect();
		let maxmind_download_url = env::var("MAXMIND_DOWNLOAD_URL").ok();
		let auto_update = maxmind_account_id.is_some() || maxmind_download_url.is_some();
		let maxmind_download_url = maxmind_download_url
			.unwrap_or_else(|| DOWNLOAD_URL.to_owned());
		let auto_update_interval = env::var("AUTO_UPDATE_INTERVAL").ok()
			.unwrap_or_else(|| "24".to_owned())
			.parse()
			.expect("AUTO_UPDATE_INTERVAL must be a valid integer");
		let api_key = env::var("API_KEY").ok();
		let recaptcha_site_key = env::var("RECAPTCHA_SITE_KEY").ok();
		let recaptcha_secret_key = env::var("RECAPTCHA_SECRET_KEY").ok();
		
		Arc::new(Self {
			listen_addr,
			data_dir,
			maxmind_account_id,
			maxmind_license_key,
			maxmind_bearer_token,
			maxmind_editions,
			maxmind_download_url,
			auto_update,
			auto_update_interval,
			api_key,
			recaptcha_site_key,
			recaptcha_secret_key,
		})
	}
}
