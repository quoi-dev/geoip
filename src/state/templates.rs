use std::sync::Arc;
use handlebars::Handlebars;
use serde_json::json;
use thiserror::Error;
use crate::config::AppConfig;
use crate::model::IndexPageCtx;

#[derive(Debug, Error)]
pub enum TemplateServiceError {
	#[error(transparent)]
	Render(#[from] handlebars::RenderError),
	
	#[error(transparent)]
	Serde(#[from] serde_json::Error),
}

pub struct TemplateService {
	config: Arc<AppConfig>,
	handlebars: Handlebars<'static>,
}

impl TemplateService {
	pub fn new(config: Arc<AppConfig>) -> Arc<Self> {
		let mut handlebars = Handlebars::new();
		handlebars.set_strict_mode(true);
		handlebars.set_dev_mode(cfg!(debug_assertions));
		handlebars.register_template_file("index", "./dist/index.html")
			.expect("Unable to register index template");
		Arc::new(Self {
			config,
			handlebars,
		})
	}
	
	pub fn index_ctx(&self) -> IndexPageCtx {
		IndexPageCtx {
			recaptcha_site_key: self.config.recaptcha_site_key.clone(),
			osm_tiles_url: self.config.osm_tiles_url.clone(),
		}
	}
	
	pub fn render_index(&self) -> Result<String, TemplateServiceError> {
		let ctx = serde_json::to_string(&self.index_ctx())?;
		let html = self.handlebars.render("index", &json!({"ctx": ctx}))?;
		Ok(html)
	}
}
