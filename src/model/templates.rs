use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexPageCtx {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub recaptcha_site_key: Option<String>,
}
