// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> miette::Result<()> {
	use std::sync::Arc;
	use twilight_sword::config::parse_config;
	use twilight_sword::database::{connect_db, run_embedded_migrations};
	use twilight_sword::discord::{run_bot, set_up_client};
	use twilight_sword::web::server::run_server_task;

	tracing_subscriber::fmt::init();

	let config = parse_config("config.kdl").await?;
	let db_connection_pool = connect_db(&config)?;
	run_embedded_migrations(&db_connection_pool)?;

	let config = Arc::new(config);

	let discord_client = set_up_client(&config);

	tokio::spawn(run_server_task(
		Arc::clone(&config),
		db_connection_pool.clone(),
		Arc::clone(&discord_client),
	));

	run_bot(db_connection_pool.clone(), Arc::clone(&config), discord_client).await?;

	Ok(())
}

#[cfg(not(feature = "ssr"))]
fn main() {}
