// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::config::ConfigDocument;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use miette::{Diagnostic, IntoDiagnostic};
use std::error::Error;
use std::fmt;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

// To get boxed errors (as returned by the migration runner) into miette, we need a wrapper type for them.
#[derive(Debug, Diagnostic)]
pub struct MigrationError(pub Box<dyn Error + Send + Sync>);

impl fmt::Display for MigrationError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(&self.0, f)
	}
}

impl Error for MigrationError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		self.0.source()
	}
}

pub fn connect_db(config: &ConfigDocument) -> miette::Result<Pool<ConnectionManager<PgConnection>>> {
	let url = db_url(config);
	let manager: ConnectionManager<PgConnection> = ConnectionManager::new(url);
	Pool::builder().test_on_check_out(true).build(manager).into_diagnostic()
}

fn db_url(config: &ConfigDocument) -> String {
	let db_config = &config.database;
	match db_config.port {
		Some(port) => format!(
			"postgres://{}:{}@{}:{}/{}",
			db_config.username, db_config.password, db_config.host, port, db_config.database
		),
		None => format!(
			"postgres://{}:{}@{}/{}",
			db_config.username, db_config.password, db_config.host, db_config.database
		),
	}
}

pub fn run_embedded_migrations(
	db_connection_pool: &Pool<ConnectionManager<PgConnection>>,
) -> Result<(), MigrationError> {
	let mut db_connection = match db_connection_pool.get() {
		Ok(connection) => connection,
		Err(error) => return Err(MigrationError(Box::new(error))),
	};
	match db_connection.run_pending_migrations(MIGRATIONS) {
		Ok(_) => Ok(()),
		Err(error) => Err(MigrationError(error)),
	}
}
