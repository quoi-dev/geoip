use std::sync::Arc;
use std::time::Instant;
use axum::extract::{Query, State};
use axum::{middleware, Json, Router};
use axum::body::{Body, Bytes};
use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::get;
use log::error;
use utoipa_swagger_ui::SwaggerUi;
use crate::extractors::ClientIp;
use crate::model::{ErrorDTO, GeoIpLookupQuery, GeoIpLookupResult, GeoIpStatus, IpDetectResult};
use crate::state::{AppState, MaxMindServiceError};

pub fn build_router(state: Arc<AppState>) -> Router {
	let openapi_spec: serde_json::Value = serde_yaml::from_str(include_str!("../openapi.yaml"))
		.expect("Unable to parse OpenAPI spec");
	
	let (
		prometheus_layer,
		metric_handle
	) = axum_prometheus::PrometheusMetricLayer::pair();
	
	Router::new()
		.route("/api/status", get(get_status))
		.route("/api/ip", get(detect_ip))
		.route("/api/lookup", get(lookup_ip))
		.route("/api/metrics", get(|| async move { metric_handle.render() }))
		.merge(
			SwaggerUi::new("/swagger-ui")
				.external_url_unchecked("/api/docs", openapi_spec)
				.config(utoipa_swagger_ui::Config::default().persist_authorization(true))
		)
		.layer(middleware::from_fn(log_internal_server_errors))
		.layer(prometheus_layer)
		.with_state(state)
}

async fn get_status(State(state): State<Arc<AppState>>) -> Json<GeoIpStatus> {
	Json(state.maxmind.status())
}

async fn detect_ip(ClientIp(client_ip): ClientIp) -> Json<IpDetectResult> {
	Json(IpDetectResult {
		ip: client_ip,
	})
}

async fn lookup_ip(
	State(state): State<Arc<AppState>>,
	ClientIp(client_ip): ClientIp,
	Query(query): Query<GeoIpLookupQuery>,
) -> Result<Json<GeoIpLookupResult>, ErrorDTO> {
	let start = Instant::now();
	let ip = query.ip.unwrap_or(client_ip);
	match state.maxmind.lookup(
		ip,
		query.locale.as_deref().unwrap_or("en"),
		query.edition.as_deref(),
	) {
		Ok(info) => {
			let elapsed = start.elapsed();
			Ok(Json(GeoIpLookupResult {
				ip,
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
