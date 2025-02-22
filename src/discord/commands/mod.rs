// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
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
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use type_map::concurrent::TypeMap;

mod close;
mod list_restricted_users;
mod reply;
mod restrict_ticket_user;
mod settings;
mod setup;
mod unrestrict_ticket_user;

pub fn command_definitions() -> Vec<Command> {
	vec![
		close::command_definition(),
		list_restricted_users::command_definition(),
		reply::command_definition(),
		restrict_ticket_user::command_definition(),
		setup::command_definition(),
		settings::command_definition(),
		unrestrict_ticket_user::command_definition(),
	]
}

pub async fn route_command(
	interaction: &InteractionCreate,
	command_data: &CommandData,
	http_client: &Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	match command_data.name.as_str() {
		"close" => close::handle_command(interaction, http_client, application_id, db_connection_pool).await,
		"list_restricted_users" => {
			list_restricted_users::handle_command(interaction, http_client, application_id, db_connection_pool).await
		}
		"reply" => reply::handle_command(interaction, http_client, application_id, db_connection_pool, bot_state).await,
		"restrict_ticket_user" => {
			restrict_ticket_user::handle_command(
				interaction,
				command_data,
				http_client,
				application_id,
				db_connection_pool,
			)
			.await
		}
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
		"unrestrict_ticket_user" => {
			unrestrict_ticket_user::handle_command(
				interaction,
				command_data,
				http_client,
				application_id,
				db_connection_pool,
			)
			.await
		}
		_ => bail!("Unknown command encountered: {}\n{:?}", command_data.name, command_data),
	}
}
