mod maxmind;

pub use maxmind::*;

use std::sync::Arc;
use reqwest::Client;
use crate::config::AppConfig;

pub struct AppState {
	#[allow(dead_code)] pub config: Arc<AppConfig>,
	#[allow(dead_code)] pub client: Client,
	pub maxmind: Arc<MaxMindService>,
}

impl AppState {
	pub fn new(config: Arc<AppConfig>) -> Arc<Self> {
		let client = Client::new();
		let maxmind = MaxMindService::new(config.clone(), client.clone());
		
		Arc::new(Self {
			config,
			client,
			maxmind,
		})
	}
}
