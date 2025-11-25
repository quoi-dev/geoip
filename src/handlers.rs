use std::sync::Arc;
use std::time::Instant;
use axum::extract::{Query, State};
use axum::{Json, Router};
use axum::http::StatusCode;
use axum::routing::get;
use crate::model::{ErrorDTO, GeoIpLookupQuery, GeoIpLookupResult, GeoIpStatus};
use crate::state::{AppState, MaxMindServiceError};

pub fn build_router(state: Arc<AppState>) -> Router {
	Router::new()
		.route("/api/status", get(get_status))
		.route("/api/lookup", get(lookup_ip))
		.with_state(state)
}

async fn get_status(State(state): State<Arc<AppState>>) -> Json<GeoIpStatus> {
	Json(state.maxmind.status())
}

async fn lookup_ip(
	State(state): State<Arc<AppState>>,
	Query(query): Query<GeoIpLookupQuery>,
) -> Result<Json<GeoIpLookupResult>, ErrorDTO> {
	let start = Instant::now();
	match state.maxmind.lookup(
		query.ip,
		query.locale.as_deref().unwrap_or("en"),
		query.edition.as_deref(),
	) {
		Ok(info) => {
			let elapsed = start.elapsed();
			Ok(Json(GeoIpLookupResult {
				ip: query.ip,
				info,
				elapsed: elapsed.as_secs_f32(),
			}))
		},
		Err(MaxMindServiceError::UnknownEdition) => Err(ErrorDTO::new_static(
			StatusCode::NOT_FOUND,
			"Unknown MaxMind database edition",
		)),
		Err(MaxMindServiceError::MissingDatabase) => Err(ErrorDTO::new_static(
			StatusCode::SERVICE_UNAVAILABLE,
			"Missing MaxMind database",
		)),
		Err(err) => Err(err.into()),
	}
}
