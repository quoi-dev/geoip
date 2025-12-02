use std::sync::Arc;
use axum::http::StatusCode;
use log::debug;
use thiserror::Error;
use crate::config::AppConfig;
use crate::model::{RecaptchaVerifyRequest, RecaptchaVerifyResponse};

const VERIFY_URL: &str = "https://www.google.com/recaptcha/api/siteverify";

#[derive(Debug, Error)]
pub enum RecaptchaServiceError {
	#[error(transparent)]
	Reqwest(#[from] reqwest::Error),
	
	#[error("HTTP error (status={0})")]
	HttpError(StatusCode),
}

pub struct RecaptchaService {
	config: Arc<AppConfig>,
	client: reqwest::Client,
}

impl RecaptchaService {
	pub fn new(config: Arc<AppConfig>, client: reqwest::Client) -> Arc<Self> {
		Arc::new(Self {
			config,
			client,
		})
	}
	
	pub async fn verify(&self, token: &str) -> Result<bool, RecaptchaServiceError> {
		let Some(secret_key) = self.config.recaptcha_secret_key.as_deref() else {
			return Ok(false);
		};
		let params = RecaptchaVerifyRequest {
			secret: secret_key.to_owned(),
			response: token.to_owned(),
			remote_ip: None,
		};
		let res = self.client.post(VERIFY_URL)
			.form(&params)
			.send()
			.await?;
		if !res.status().is_success() {
			return Err(RecaptchaServiceError::HttpError(res.status()));
		}
		let res: RecaptchaVerifyResponse = res.json().await?;
		debug!(
			"Verify response: success={}, score={:?}, action={:?}, \
			challenge_ts={:?}, hostname={:?}, apk_package_name={:?}, \
			error_codes={:?}",
			res.success,
			res.score,
			res.action,
			res.challenge_ts,
			res.hostname,
			res.apk_package_name,
			res.error_codes,
		);
		Ok(res.success)
	}
}
