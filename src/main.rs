// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Arc;

mod config;
mod database;
mod discord;
mod model;
mod schema;

use database::{connect_db, run_embedded_migrations};
use discord::run_bot;

#[tokio::main]
async fn main() -> miette::Result<()> {
	let config = config::parse_config("config.kdl").await?;
	let db_connection_pool = connect_db(&config)?;
	run_embedded_migrations(&db_connection_pool)?;

	let config = Arc::new(config);

	run_bot(db_connection_pool.clone(), Arc::clone(&config)).await?;

	Ok(())
}
