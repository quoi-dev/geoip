use std::sync::Arc;
use std::time::Instant;
use ahash::AHashMap;
use axum::extract::{Path, Query, State};
use axum::{middleware, Json, Router};
use axum::body::{Body, Bytes};
use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum_extra::headers::IfModifiedSince;
use axum_extra::TypedHeader;
use log::error;
use metrics::histogram;
use tower_http::services::{ServeDir, ServeFile};
use utoipa_swagger_ui::SwaggerUi;
use crate::extractors::{ApiKeyAuth, ApiKeyOrRecaptchaAuth, ClientIp};
use crate::model::{ErrorDTO, GeoIpLookupQuery, GeoIpLookupResult, GeoIpStatus, IndexPageCtx, IpDetectResult};
use crate::state::{AppState, MaxMindServiceError};

pub fn build_router(state: Arc<AppState>) -> Router {
	let openapi_spec: serde_json::Value = serde_yaml::from_str(include_str!("../openapi.yaml"))
		.expect("Unable to parse OpenAPI spec");
	
	let (
		prometheus_layer,
		metric_handle
	) = axum_prometheus::PrometheusMetricLayer::pair();
	
	Router::new()
		.route("/api/ctx", get(get_index_page_ctx))
		.route("/api/status", get(get_status))
		.route("/api/ip", get(detect_ip))
		.route("/api/geoip", get(lookup_geoip))
		.route("/api/timezones", get(get_all_timezones))
		.route("/api/metrics", get(|| async move { metric_handle.render() }))
		.merge(
			SwaggerUi::new("/swagger-ui")
				.external_url_unchecked("/api/docs", openapi_spec)
				.config(utoipa_swagger_ui::Config::default()
					.display_operation_id(true)
					.persist_authorization(true)
				)
		)
		.route("/", get(get_index_page))
		.route_service("/favicon.ico", ServeFile::new("dist/favicon.ico"))
		.nest_service("/static", ServeDir::new("dist/static").precompressed_gzip())
		.route("/files/mmdb/{edition}", get(download_mmdb_archive_file))
		.layer(middleware::from_fn(log_internal_server_errors))
		.layer(prometheus_layer)
		.with_state(state)
}

async fn get_index_page(State(state): State<Arc<AppState>>) -> Result<Html<String>, ErrorDTO> {
	let html = state.templates.render_index()?;
	Ok(Html(html))
}

async fn get_index_page_ctx(State(state): State<Arc<AppState>>) -> Json<IndexPageCtx> {
	Json(state.templates.index_ctx())
}

async fn get_status(State(state): State<Arc<AppState>>) -> Json<GeoIpStatus> {
	Json(state.maxmind.status())
}

async fn download_mmdb_archive_file(
	State(state): State<Arc<AppState>>,
	_auth: ApiKeyAuth,
	Path(edition): Path<String>,
	if_modified_since: Option<TypedHeader<IfModifiedSince>>,
) -> Result<axum::response::Response, ErrorDTO> {
	let info = state.maxmind.get_archive(&edition)?;
	let res = state.files.download_archive(
		info, 
		&format!("{}.tar.gz", edition),
		if_modified_since,
	).await?;
	Ok(res)
}

async fn detect_ip(ClientIp(client_ip): ClientIp) -> Json<IpDetectResult> {
	Json(IpDetectResult {
		ip: client_ip,
	})
}

async fn lookup_geoip(
	State(state): State<Arc<AppState>>,
	ClientIp(client_ip): ClientIp,
	_auth: ApiKeyOrRecaptchaAuth,
	Query(query): Query<GeoIpLookupQuery>,
) -> Result<Json<GeoIpLookupResult>, ErrorDTO> {
	let start = Instant::now();
	let ip = query.ip.unwrap_or(client_ip);
	let locale = query.locale.as_deref().unwrap_or("en");
	let edition = query.edition.as_deref().or_else(|| state.maxmind.default_edition());
	match state.maxmind.lookup(ip, locale, edition) {
		Ok(info) => {
			let elapsed = start.elapsed();
			histogram!(
				"lookup_duration_seconds",
				"edition" => edition.unwrap_or("Unknown").to_owned(),
			).record(elapsed.as_secs_f64());
			Ok(Json(GeoIpLookupResult {
				ip,
				info,
				elapsed: elapsed.as_secs_f64(),
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

async fn get_all_timezones(
	State(state): State<Arc<AppState>>,
	_auth: ApiKeyAuth,
) -> Json<AHashMap<String, String>> {
	Json(state.timezones.get_all())
}

async fn log_internal_server_errors(req: Request<Body>, next: Next) -> Response<Body> {
	let res = next.run(req).await;
	if res.status() != StatusCode::INTERNAL_SERVER_ERROR {
		return res;
	}
	let headers = res.headers().clone();
	let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
		.await
		.unwrap_or_else(|err| {
			error!("Unable to read body of failed response: {err}");
			Bytes::default()
		});
	match serde_json::from_slice::<ErrorDTO>(&body_bytes) {
		Ok(res) => error!("Internal Server Error: {}", res.error),
		Err(_) => error!("Internal Server Error")
	}
	(StatusCode::INTERNAL_SERVER_ERROR, headers, body_bytes).into_response()
}
