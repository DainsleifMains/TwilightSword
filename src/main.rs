// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use miette::IntoDiagnostic;
use serenity::prelude::*;

mod config;
mod database;
mod discord;

use database::{connect_db, run_embedded_migrations, DatabaseConnection};

#[tokio::main]
async fn main() -> miette::Result<()> {
	let config = config::parse_config("config.kdl").await?;
	let db_connection_pool = connect_db(&config)?;
	run_embedded_migrations(&db_connection_pool)?;

	let intents = GatewayIntents::empty();
	let client_builder = Client::builder(&config.discord_token, intents)
		.event_handler(discord::Handler)
		.type_map_insert::<DatabaseConnection>(db_connection_pool);
	let mut client = client_builder.await.into_diagnostic()?;

	client.start().await.into_diagnostic()?;

	Ok(())
}
