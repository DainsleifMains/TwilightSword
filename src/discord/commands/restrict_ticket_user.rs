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
use diesel::result::{DatabaseErrorKind, Error as DbError};
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
	let restrict_user = UserBuilder::new("restrict_user", "The user to restrict from creating tickets")
		.required(true)
		.build();
	CommandBuilder::new(
		"restrict_ticket_user",
		"Restrict a user from submitting tickets on this server",
		CommandType::ChatInput,
	)
	.contexts([InteractionContextType::Guild])
	.default_member_permissions(Permissions::MODERATE_MEMBERS)
	.option(restrict_user)
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
		bail!("Restrict ticket user command was used outside of a guild");
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
		bail!("Restrict ticket user command received without required options");
	};
	if option.name != "restrict_user" {
		bail!("Restrict ticket user command received without required option restrict_user");
	}
	let CommandOptionValue::User(restrict_user) = option.value else {
		bail!("Restrict ticket user argument restrict_user wasn't a user");
	};

	let db_restrict_user = database_id_from_discord_id(restrict_user.get());

	let new_restriction = TicketRestrictedUser {
		guild_id: db_guild_id,
		user_id: db_restrict_user,
	};
	let insert_result = diesel::insert_into(ticket_restricted_users::table)
		.values(new_restriction)
		.execute(&mut db_connection);

	let response = match insert_result {
		Ok(_) => InteractionResponseDataBuilder::new()
			.content(format!(
				"{} is now restricted from sending tickets.",
				restrict_user.mention()
			))
			.allowed_mentions(AllowedMentions::default())
			.build(),
		Err(DbError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => InteractionResponseDataBuilder::new()
			.content(format!(
				"{} was already restricted from sending tickets.",
				restrict_user.mention()
			))
			.flags(MessageFlags::EPHEMERAL)
			.allowed_mentions(AllowedMentions::default())
			.build(),
		Err(error) => bail!(error),
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
