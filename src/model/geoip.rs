use std::net::IpAddr;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct GeoIpStatus {
	pub databases: Vec<GeoIpDatabaseStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeoIpDatabaseStatus {
	pub edition: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IpDetectResult {
	pub ip: IpAddr,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeoIpLookupQuery {
	pub ip: Option<IpAddr>,
	pub locale: Option<String>,
	pub edition: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeoIpLookupResult {
	pub ip: IpAddr,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub info: Option<GeoIpInfo>,
	pub elapsed: f32,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct GeoIpInfo {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub continent_id: Option<u32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub continent_code: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub continent_name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country_id: Option<u32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country_iso_code: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country_name: Option<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub subdivisions: Vec<GeoNameSubdivision>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub city_id: Option<u32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub city_name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metro_code: Option<u16>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub postal_code: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timezone: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub latitude: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub longitude: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub accuracy_radius: Option<u16>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_in_european_union: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_anonymous_proxy: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_anycast: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_satellite_provider: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub autonomous_system_number: Option<u32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub autonomous_system_organization: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeoNameSubdivision {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<u32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub iso_code: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
}
