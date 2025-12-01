use std::sync::Arc;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use axum_extra::TypedHeader;
use crate::state::AppState;

pub struct Auth;

impl FromRequestParts<Arc<AppState>> for Auth {
	type Rejection = StatusCode;
	
	async fn from_request_parts(
		parts: &mut Parts,
		state: &Arc<AppState>,
	) -> Result<Self, Self::Rejection> {
		let Some(token) = &state.config.api_key else { return Ok(Auth) };
		let auth = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
			.await
			.map_err(|_| StatusCode::UNAUTHORIZED)?;
		if auth.token() != token {
			return Err(StatusCode::UNAUTHORIZED);
		}
		Ok(Auth)
	}
}
