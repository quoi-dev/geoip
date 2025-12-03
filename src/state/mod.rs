mod maxmind;
mod templates;
mod recaptcha;
mod timezones;
mod files;

pub use maxmind::*;
pub use templates::*;
pub use recaptcha::*;
pub use timezones::*;
pub use files::*;

use std::sync::Arc;
use reqwest::Client;
use crate::config::AppConfig;

pub struct AppState {
	#[allow(dead_code)] pub config: Arc<AppConfig>,
	#[allow(dead_code)] pub client: Client,
	pub timezones: Arc<TimezoneService>,
	pub maxmind: Arc<MaxMindService>,
	pub templates: Arc<TemplateService>,
	pub recaptcha: Arc<RecaptchaService>,
	pub files: Arc<FileService>,
}

impl AppState {
	pub async fn new(config: Arc<AppConfig>) -> Arc<Self> {
		let client = Client::new();
		let files = FileService::new(config.clone(), client.clone()).await;
		let timezones = TimezoneService::new(config.clone(), files.clone()).await;
		let maxmind = MaxMindService::new(
			config.clone(),
			files.clone(),
			timezones.clone(),
		).await;
		let templates = TemplateService::new(config.clone());
		let recaptcha = RecaptchaService::new(config.clone(), client.clone());
		
		Arc::new(Self {
			config,
			client,
			timezones,
			maxmind,
			templates,
			recaptcha,
			files,
		})
	}
}
