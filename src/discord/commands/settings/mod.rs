// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::bail;
use std::sync::Arc;
use twilight_http::client::Client;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::guild::Permissions;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use twilight_util::builder::command::CommandBuilder;

mod admin_role;
mod staff_role;

pub fn command_definition() -> Command {
	CommandBuilder::new(
		"settings",
		"View or modify settings for your server",
		CommandType::ChatInput,
	)
	.dm_permission(false)
	.default_member_permissions(Permissions::MANAGE_GUILD)
	.option(admin_role::subcommand_definition())
	.option(staff_role::subcommand_definition())
	.build()
}

pub async fn handle_command(
	interaction: &InteractionCreate,
	command_data: &CommandData,
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> miette::Result<()> {
	let Some(subcommand_data) = command_data.options.first() else {
		bail!("Settings command invoked with no subcommand");
	};

	match subcommand_data.name.as_str() {
		"admin_role" => {
			admin_role::handle_subcommand(
				interaction,
				&subcommand_data.value,
				http_client,
				application_id,
				db_connection_pool,
			)
			.await
		}
		"staff_role" => {
			staff_role::handle_subcommand(
				interaction,
				&subcommand_data.value,
				http_client,
				application_id,
				db_connection_pool,
			)
			.await
		}
		_ => unimplemented!(),
	}
}
