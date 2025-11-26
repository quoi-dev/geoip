use std::net::SocketAddr;
use log::info;
use tokio::net::TcpListener;
use crate::config::AppConfig;
use crate::handlers::build_router;
use crate::state::AppState;

mod config;
mod handlers;
mod state;
mod model;
mod extractors;

#[tokio::main]
async fn main() {
	let _ = dotenvy::dotenv();
	env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
	let config = AppConfig::load_from_env();
	let state = AppState::new(config.clone());
	state.maxmind.start_updater();
	let router = build_router(state.clone());
	let listener = TcpListener::bind(config.listen_addr)
		.await
		.expect("Unable to bind TCP listener");
	info!("Listening on http://{}/", listener.local_addr().expect("Unable to get local address"));
	axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
		.await
		.expect("Unable to start Axum server");
}
