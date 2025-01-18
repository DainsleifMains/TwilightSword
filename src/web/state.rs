// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::config::ConfigData;
use axum::extract::FromRef;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use leptos::config::LeptosOptions;
use std::sync::Arc;
use twilight_http::client::Client;

#[derive(Clone, Debug, FromRef)]
pub struct AppState {
	pub leptos_options: LeptosOptions,
	pub config: Arc<ConfigData>,
	pub db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	pub discord_client: Arc<Client>,
}
