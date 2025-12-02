mod maxmind;
mod templates;
mod recaptcha;
mod timezones;

pub use maxmind::*;
pub use templates::*;
pub use recaptcha::*;
pub use timezones::*;

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
}

impl AppState {
	pub fn new(config: Arc<AppConfig>) -> Arc<Self> {
		let client = Client::new();
		let timezones = TimezoneService::new();
		let maxmind = MaxMindService::new(
			config.clone(), 
			client.clone(),
			timezones.clone(),
		);
		let templates = TemplateService::new(config.clone());
		let recaptcha = RecaptchaService::new(config.clone(), client.clone());
		
		Arc::new(Self {
			config,
			client,
			timezones,
			maxmind,
			templates,
			recaptcha,
		})
	}
}
