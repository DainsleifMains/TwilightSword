// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::utils::setup::NOT_SET_UP_FOR_GUILD;
use crate::model::{Guild, database_id_from_discord_id};
use crate::schema::{guilds, ticket_restricted_users};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{IntoDiagnostic, bail};
use twilight_http::client::Client;
use twilight_mention::fmt::Mention;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::application::interaction::application_command::{CommandData, CommandOptionValue};
use twilight_model::channel::message::{AllowedMentions, MessageFlags};
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use twilight_util::builder::InteractionResponseDataBuilder;
use twilight_util::builder::command::{CommandBuilder, UserBuilder};

pub fn command_definition() -> Command {
	let unrestrict_user = UserBuilder::new("unrestrict_user", "The user to allow to create tickets")
		.required(true)
		.build();
	CommandBuilder::new(
		"unrestrict_ticket_user",
		"Removes a restriction from a user, once again allowing them to submit tickets on this server",
		CommandType::ChatInput,
	)
	.contexts([InteractionContextType::Guild])
	.default_member_permissions(Permissions::MODERATE_MEMBERS)
	.option(unrestrict_user)
	.build()
}

pub async fn handle_command(
	interaction: &InteractionCreate,
	command_data: &CommandData,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> miette::Result<()> {
	let Some(guild) = interaction.guild_id else {
		bail!("Unrestrict ticket user command was used outside of a guild");
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

	let Some(option) = command_data.options.first() else {
		bail!("Unrestrict ticket user command received without required options");
	};
	if option.name != "unrestrict_user" {
		bail!("Unrestrict ticket user command received without required option unrestrict_user");
	}
	let CommandOptionValue::User(unrestrict_user) = option.value else {
		bail!("Unrestrict ticket user argument unrestrict_user wasn't a user");
	};

	let db_unrestrict_user = database_id_from_discord_id(unrestrict_user.get());

	diesel::delete(ticket_restricted_users::table)
		.filter(
			ticket_restricted_users::guild_id
				.eq(db_guild_id)
				.and(ticket_restricted_users::user_id.eq(db_unrestrict_user)),
		)
		.execute(&mut db_connection)
		.into_diagnostic()?;

	let response = InteractionResponseDataBuilder::new()
		.content(format!(
			"{} is now not restricted from sending tickets.",
			unrestrict_user.mention()
		))
		.allowed_mentions(AllowedMentions::default())
		.build();
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
