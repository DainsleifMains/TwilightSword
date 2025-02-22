// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::utils::setup::NOT_SET_UP_FOR_GUILD;
use crate::model::{Guild, TicketRestrictedUser, database_id_from_discord_id};
use crate::schema::{guilds, ticket_restricted_users};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{IntoDiagnostic, bail};
use twilight_http::client::Client;
use twilight_mention::fmt::Mention;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::channel::message::{AllowedMentions, MessageFlags};
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use twilight_util::builder::InteractionResponseDataBuilder;
use twilight_util::builder::command::CommandBuilder;

pub fn command_definition() -> Command {
	CommandBuilder::new(
		"list_restricted_users",
		"Lists users restricted from sending tickets on this server",
		CommandType::ChatInput,
	)
	.contexts([InteractionContextType::Guild])
	.default_member_permissions(Permissions::MODERATE_MEMBERS)
	.build()
}

pub async fn handle_command(
	interaction: &InteractionCreate,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> miette::Result<()> {
	let Some(guild) = interaction.guild_id else {
		bail!("List restricted users command was used outside of a guild");
	};

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;

	let db_guild_id = database_id_from_discord_id(guild.get());
	let guild: Option<Guild> = guilds::table
		.find(db_guild_id)
		.first(&mut db_connection)
		.optional()
		.into_diagnostic()?;

	let interaction_client = http_client.interaction(application_id);

	if guild.is_none() {
		let response = InteractionResponseDataBuilder::new()
			.content(NOT_SET_UP_FOR_GUILD)
			.flags(MessageFlags::EPHEMERAL)
			.build();
		let response = InteractionResponse {
			kind: InteractionResponseType::ChannelMessageWithSource,
			data: Some(response),
		};
		interaction_client
			.create_response(interaction.id, &interaction.token, &response)
			.await
			.into_diagnostic()?;
		return Ok(());
	}

	let restricted_users: Vec<TicketRestrictedUser> = ticket_restricted_users::table
		.filter(ticket_restricted_users::guild_id.eq(db_guild_id))
		.load(&mut db_connection)
		.into_diagnostic()?;
	let restricted_user_text: Vec<String> = restricted_users
		.into_iter()
		.map(|user_data| format!("- {}", user_data.get_user_id().mention()))
		.collect();

	let response = if restricted_user_text.is_empty() {
		InteractionResponseDataBuilder::new()
			.content("No users are restricted from sending tickets.")
			.build()
	} else {
		let message = format!(
			"The following users are restricted from sending tickets:\n{}",
			restricted_user_text.join("\n")
		);
		InteractionResponseDataBuilder::new()
			.content(message)
			.allowed_mentions(AllowedMentions::default())
			.build()
	};
	let response = InteractionResponse {
		kind: InteractionResponseType::ChannelMessageWithSource,
		data: Some(response),
	};
	interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.await
		.into_diagnostic()?;

	Ok(())
}
