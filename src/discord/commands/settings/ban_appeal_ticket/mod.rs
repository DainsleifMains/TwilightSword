// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::utils::setup::NOT_SET_UP_FOR_GUILD;
use crate::model::{Guild, database_id_from_discord_id};
use crate::schema::guilds;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{IntoDiagnostic, bail};
use std::sync::Arc;
use tokio::sync::RwLock;
use twilight_http::client::Client;
use twilight_model::application::command::CommandOption;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::ChannelType;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use twilight_util::builder::InteractionResponseDataBuilder;
use twilight_util::builder::command::{ChannelBuilder, SubCommandBuilder, SubCommandGroupBuilder};
use type_map::concurrent::TypeMap;

mod channel_get;
mod channel_set;
mod channel_unset;
mod form_get;
mod form_set;
mod form_unset;

pub fn subcommand_definition() -> CommandOption {
	let channel_option = ChannelBuilder::new("ban_appeal_ticket_channel", "Ban appeal tickets settings")
		.channel_types([ChannelType::GuildForum])
		.required(true)
		.build();

	let channel_get = SubCommandBuilder::new("channel_get", "Gets the ban appeal ticket channel");
	let channel_set =
		SubCommandBuilder::new("channel_set", "Sets the ban appeal ticket channel").option(channel_option);
	let channel_unset = SubCommandBuilder::new("channel_unset", "Removes the ban appeal ticket channel");

	let form_get = SubCommandBuilder::new("form_get", "Gets the ban appeal ticket form");
	let form_set = SubCommandBuilder::new("form_set", "Sets the ban appeal ticket form");
	let form_unset = SubCommandBuilder::new("form_unset", "Removes the ban appeal ticket form");

	SubCommandGroupBuilder::new("ban_appeal_ticket", "Ban appeal tickets settings")
		.subcommands([channel_get, channel_set, channel_unset, form_get, form_set, form_unset])
		.build()
}

pub async fn handle_subcommand(
	interaction: &InteractionCreate,
	subcommand_value: &CommandOptionValue,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let Some(guild_id) = interaction.guild_id else {
		bail!("Settings command was used outside of a guild");
	};

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;
	let db_guild_id = database_id_from_discord_id(guild_id.get());
	let guild: QueryResult<Option<Guild>> = guilds::table.find(db_guild_id).first(&mut db_connection).optional();

	let interaction_client = http_client.interaction(application_id);

	let guild = match guild {
		Ok(Some(guild)) => guild,
		Ok(None) => {
			let response = InteractionResponseDataBuilder::new()
				.content(NOT_SET_UP_FOR_GUILD)
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
		Err(error) => {
			tracing::error!(source = ?error, "Failed to retrieve guild for getting or updating ban appeal ticket settings");
			let response = InteractionResponseDataBuilder::new()
				.content("An internal error occurred handling this command.")
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
	};

	let CommandOptionValue::SubCommandGroup(value_data) = subcommand_value else {
		bail!("Command data is malformed; expected `/settings ban_appeal_ticket` to get a subcommand group value");
	};
	let Some(value) = value_data.first() else {
		bail!("Command data is malformed; expected `/settings ban_appeal_ticket` to have a subcommand");
	};
	match value.name.as_str() {
		"channel_get" => channel_get::execute(interaction, &guild, http_client, application_id).await,
		"channel_set" => {
			channel_set::execute(
				interaction,
				guild_id,
				&guild,
				&value.value,
				http_client,
				application_id,
				&mut db_connection,
			)
			.await
		}
		"channel_unset" => {
			channel_unset::execute(interaction, &guild, http_client, application_id, &mut db_connection).await
		}
		"form_get" => form_get::execute(interaction, &guild, http_client, application_id, &mut db_connection).await,
		"form_set" => {
			form_set::execute(
				interaction,
				guild_id,
				&guild,
				http_client,
				application_id,
				&mut db_connection,
				bot_state,
			)
			.await
		}
		"form_unset" => form_unset::execute(interaction, &guild, http_client, application_id, &mut db_connection).await,
		_ => bail!(
			"Unknown settings ban_appeal_ticket_channel subcommand encountered: {}\n{:?}",
			value.name,
			subcommand_value
		),
	}
}
