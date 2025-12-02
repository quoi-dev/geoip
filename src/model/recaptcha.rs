use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct RecaptchaVerifyRequest {
	pub secret: String,
	pub response: String,
	#[serde(rename = "remoteip", skip_serializing_if = "Option::is_none")]
	pub remote_ip: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecaptchaVerifyResponse {
	pub success: bool,
	pub score: Option<f32>,
	pub action: Option<String>,
	pub challenge_ts: Option<String>,
	pub hostname: Option<String>,
	pub apk_package_name: Option<String>,
	#[serde(default, rename = "error-codes")]
	pub error_codes: Vec<String>,
}
