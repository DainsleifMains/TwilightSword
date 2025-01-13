// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> miette::Result<()> {
	use axum::Router;
	use leptos::logging::log;
	use leptos::prelude::*;
	use leptos_axum::{generate_route_list, LeptosRoutes};
	use miette::IntoDiagnostic;
	use std::sync::Arc;
	use tokio::net::TcpListener;
	use twilight_sword::config::parse_config;
	use twilight_sword::database::{connect_db, run_embedded_migrations};
	use twilight_sword::discord::run_bot;
	use twilight_sword::web::app::{shell, App};

	tracing_subscriber::fmt::init();

	let config = parse_config("config.kdl").await?;
	let db_connection_pool = connect_db(&config)?;
	run_embedded_migrations(&db_connection_pool)?;

	let config = Arc::new(config);

	let web_config = get_configuration(None).into_diagnostic()?;
	let site_addr = &config.web_addr;
	let leptos_options = web_config.leptos_options;
	let routes = generate_route_list(App);

	let app = Router::new()
		.leptos_routes(&leptos_options, routes, {
			let leptos_options = leptos_options.clone();
			move || shell(leptos_options.clone())
		})
		.fallback(leptos_axum::file_and_error_handler(shell))
		.with_state(leptos_options);

	log!("Listening on http://{}", &site_addr);
	let listener = TcpListener::bind(&site_addr).await.into_diagnostic()?;
	tokio::spawn(async {
		let run_result = axum::serve(listener, app.into_make_service()).await;
		if let Err(error) = run_result {
			log!("Web server run error: {:?}", error);
		}
	});

	run_bot(db_connection_pool.clone(), Arc::clone(&config)).await?;

	Ok(())
}

#[cfg(not(feature = "ssr"))]
fn main() {}
