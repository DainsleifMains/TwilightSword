// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::auth::{discord_auth_layer, discord_auth_route};
use super::pages::app::App;
use super::pages::shell::shell;
use super::session::DatabaseStore;
use super::state::AppState;
use crate::config::ConfigData;
use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{StatusCode, Uri};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use leptos::logging::log;
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list, render_app_to_stream};
use miette::IntoDiagnostic;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower::util::ServiceExt;
use tower_http::services::ServeDir;
use tower_sessions::cookie::SameSite;
use tower_sessions::service::SessionManagerLayer;
use twilight_http::client::Client;

pub async fn run_server_task(
	config: Arc<ConfigData>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	discord_client: Arc<Client>,
) {
	let task_result = run_server(config, db_connection_pool, discord_client).await;
	if let Err(error) = task_result {
		tracing::error!(source = ?error, "Web server failed to run");
	}
}

async fn run_server(
	config: Arc<ConfigData>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	discord_client: Arc<Client>,
) -> miette::Result<()> {
	let web_config = get_configuration(None).into_diagnostic()?;
	let site_addr = &config.web.bind_addr;
	let leptos_options = web_config.leptos_options;
	let routes = generate_route_list(App);

	let session_store = DatabaseStore::new(db_connection_pool.clone());
	let session_layer = SessionManagerLayer::new(session_store).with_same_site(SameSite::Lax);

	let app_state = AppState {
		leptos_options,
		config: Arc::clone(&config),
		db_connection_pool: db_connection_pool.clone(),
		discord_client: Arc::clone(&discord_client),
	};

	let app = Router::new()
		.leptos_routes_with_context(
			&app_state,
			routes,
			{
				let app_state = app_state.clone();
				move || provide_context(app_state.clone())
			},
			{
				let leptos_options = app_state.leptos_options.clone();
				move || shell(leptos_options.clone())
			},
		)
		.route("/discord_auth_callback", get(discord_auth_route))
		.fallback(file_and_error_handler)
		.layer(
			ServiceBuilder::new()
				.layer(session_layer)
				.layer(from_fn_with_state(app_state.clone(), discord_auth_layer)),
		)
		.with_state(app_state);

	log!("Listening on http://{}", &site_addr);
	let listener = TcpListener::bind(&site_addr).await.into_diagnostic()?;
	axum::serve(listener, app.into_make_service()).await.into_diagnostic()?;

	Ok(())
}

async fn file_and_error_handler(uri: Uri, State(state): State<AppState>, request: Request) -> Response {
	let site_root_dir = state.leptos_options.site_root.clone();
	let response = get_static_file(uri.clone(), &site_root_dir).await;
	let response = match response {
		Ok(response) => response,
		Err(error) => return error.into_response(),
	};

	if response.status() == StatusCode::OK {
		response.into_response()
	} else {
		let handler = render_app_to_stream(App);
		handler(request).await.into_response()
	}
}

async fn get_static_file(uri: Uri, root: &str) -> Result<Response, StatusCode> {
	let Ok(request) = Request::builder().uri(uri.clone()).body(Body::empty()) else {
		return Err(StatusCode::INTERNAL_SERVER_ERROR);
	};

	match ServeDir::new(root).oneshot(request).await {
		Ok(response) => Ok(response.into_response()),
		Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
	}
}
