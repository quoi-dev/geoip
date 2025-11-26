use std::net::{IpAddr, SocketAddr};
use axum::extract::{ConnectInfo, FromRequestParts};
use axum::extract::connect_info::MockConnectInfo;
use axum::http::request::Parts;
use axum::http::StatusCode;

#[derive(Debug, Clone, Copy)]
pub struct ClientIp(pub IpAddr);

impl<S: Send + Sync> FromRequestParts<S> for ClientIp {
	type Rejection = StatusCode;
	
	async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
		let mut ip = parts.headers.get("cf-connecting-ip")
			.and_then(|v| v.to_str().ok())
			.and_then(|v| v.parse::<IpAddr>().ok());
		if ip.is_none() {
			ip = parts.headers.get("x-forwarded-for")
				.and_then(|v| v.to_str().ok())
				.and_then(|v| v.split(',').next())
				.and_then(|v| v.trim().parse::<IpAddr>().ok());
		}
		if ip.is_none() {
			ip = parts.extensions.get::<ConnectInfo<SocketAddr>>()
				.map(|v| v.ip());
		}
		if ip.is_none() {
			ip = parts.extensions.get::<MockConnectInfo<SocketAddr>>()
				.map(|v| v.0.ip());
		}
		let ip = ip.ok_or(StatusCode::BAD_REQUEST)?;
		Ok(Self(ip))
	}
}
