use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ErrorDTO {
	pub status: u16,
	pub error: String,
}

impl ErrorDTO {
	pub fn new(status: StatusCode, error: String) -> Self {
		Self {
			status: status.as_u16(),
			error,
		}
	}
	
	pub fn new_static(status: StatusCode, error: &str) -> Self {
		Self::new(status, error.to_owned())
	}
}

impl<E: std::error::Error> From<E> for ErrorDTO {
	fn from(err: E) -> Self {
		Self::new(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
	}
}

impl IntoResponse for ErrorDTO {
	fn into_response(self) -> Response {
		(
			StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
			Json(self),
		).into_response()
	}
}