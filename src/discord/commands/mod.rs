// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::bail;
use std::sync::Arc;
use tokio::sync::RwLock;
use twilight_http::client::Client;
use twilight_model::application::command::Command;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use type_map::concurrent::TypeMap;

mod settings;
mod setup;

pub fn command_definitions() -> Vec<Command> {
	vec![setup::command_definition(), settings::command_definition()]
}

pub async fn route_command(
	interaction: &InteractionCreate,
	command_data: &CommandData,
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	match command_data.name.as_str() {
		"setup" => setup::handle_command(interaction, http_client, application_id, db_connection_pool, bot_state).await,
		"settings" => {
			settings::handle_command(
				interaction,
				command_data,
				http_client,
				application_id,
				db_connection_pool,
				bot_state,
			)
			.await
		}
		_ => bail!("Unknown command encoutered: {}\n{:?}", command_data.name, command_data),
	}
}
