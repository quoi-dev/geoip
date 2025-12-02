use std::marker::PhantomData;
use std::sync::Arc;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use axum_extra::TypedHeader;
use constant_time_eq::constant_time_eq;
use log::error;
use crate::state::AppState;

pub trait AuthMode {
	const ACCEPT_API_KEY: bool;
	const ACCEPT_RECAPTCHA_TOKEN: bool;
}

pub struct ApiKeyOrRecaptchaAuthMode;

impl AuthMode for ApiKeyOrRecaptchaAuthMode {
	const ACCEPT_API_KEY: bool = true;
	const ACCEPT_RECAPTCHA_TOKEN: bool = true;
}

pub struct Auth<Mode: AuthMode> {
	_phantom: PhantomData<Mode>,
}

pub type ApiKeyOrRecaptchaAuth = Auth<ApiKeyOrRecaptchaAuthMode>;

impl<Mode: AuthMode> FromRequestParts<Arc<AppState>> for Auth<Mode> {
	type Rejection = StatusCode;
	
	async fn from_request_parts(
		parts: &mut Parts,
		state: &Arc<AppState>,
	) -> Result<Self, Self::Rejection> {
		if Mode::ACCEPT_API_KEY {
			let Some(expected_token) = &state.config.api_key else {
				return Ok(Self { _phantom: PhantomData });
			};
			let auth = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
				.await
				.ok();
			if let Some(auth) = auth {
				if constant_time_eq(auth.token().as_bytes(), expected_token.as_bytes()) {
					return Ok(Self { _phantom: PhantomData });
				}
			}
		}
		if Mode::ACCEPT_RECAPTCHA_TOKEN {
			let token = parts.headers.get("x-recaptcha-token")
				.and_then(|v| v.to_str().ok());
			if let Some(token) = token {
				match state.recaptcha.verify(token).await {
					Ok(true) => return Ok(Self { _phantom: PhantomData }),
					Ok(false) => {}
					Err(err) => {
						error!("Unable to verify recaptcha token: {err}");
					},
				}
			}
		}
		Err(StatusCode::UNAUTHORIZED)
	}
}
