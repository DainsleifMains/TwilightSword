// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::utils::responses::NOT_SET_UP_FOR_GUILD;
use crate::discord::utils::shared_components::new_ticket_button;
use crate::model::{database_id_from_discord_id, Guild};
use crate::schema::guilds;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{bail, ensure, IntoDiagnostic};
use twilight_http::client::Client;
use twilight_http::request::AuditLogReason;
use twilight_mention::fmt::Mention;
use twilight_model::application::command::CommandOption;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::message::{AllowedMentions, MessageFlags};
use twilight_model::channel::ChannelType;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::{ApplicationMarker, GuildMarker};
use twilight_model::id::Id;
use twilight_util::builder::command::{ChannelBuilder, SubCommandBuilder, SubCommandGroupBuilder};
use twilight_util::builder::InteractionResponseDataBuilder;

pub fn subcommand_definition() -> CommandOption {
	let ticket_channel = ChannelBuilder::new(
		"start_ticket_channel",
		"The channel to which the \"create a ticket\" message is published",
	)
	.channel_types([ChannelType::GuildText])
	.required(true)
	.build();

	let get = SubCommandBuilder::new("get", "Gets the start ticket channel");
	let set = SubCommandBuilder::new("set", "Sets the start ticket channel").option(ticket_channel);
	let unset = SubCommandBuilder::new("unset", "Removes the start ticket channel");

	SubCommandGroupBuilder::new(
		"start_ticket_channel",
		"Manages the channel to which the \"create a ticket\" message is published",
	)
	.subcommands([get, set, unset])
	.build()
}

pub async fn handle_subcommand(
	interaction: &InteractionCreate,
	subcommand_value: &CommandOptionValue,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
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
			tracing::error!(source = ?error, "Failed to retrieve guild for getting or updating start ticket channel");
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
		bail!("Command data is malformed; expected `/settings start_ticket_channel` to get a subcommand group value");
	};
	let Some(value) = value_data.first() else {
		bail!("Command data is malformed; expected `/settings start_ticket_channel to have a subcommand");
	};
	match value.name.as_str() {
		"get" => get_ticket_channel(interaction, &guild, http_client, application_id).await,
		"set" => {
			set_ticket_channel(
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
		"unset" => unset_ticket_channel(interaction, &guild, http_client, application_id, &mut db_connection).await,
		_ => bail!(
			"Unknown settings start_ticket_channel subcommand encountered: {}\n{:?}",
			value.name,
			subcommand_value
		),
	}
}

async fn get_ticket_channel(
	interaction: &InteractionCreate,
	guild: &Guild,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
) -> miette::Result<()> {
	let channel = guild.get_start_ticket_channel();

	let interaction_client = http_client.interaction(application_id);
	let response_content = match channel {
		Some(channel) => format!("The start ticket channel is set up as {}.", channel.mention()),
		None => String::from("No start ticket channel is set."),
	};
	let response = InteractionResponseDataBuilder::new()
		.content(response_content)
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

async fn set_ticket_channel(
	interaction: &InteractionCreate,
	guild_id: Id<GuildMarker>,
	guild: &Guild,
	subcommand_value: &CommandOptionValue,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection: &mut PgConnection,
) -> miette::Result<()> {
	let CommandOptionValue::SubCommand(values) = subcommand_value else {
		bail!("Command data is malformed; expected `/settings start_ticket_channel set` to get subcommand data");
	};
	let Some(start_ticket_channel) = values.first() else {
		bail!("Command data is malformed; expected `/settings start_ticket_channel set` to have required option `start_ticket_channel`");
	};
	ensure!(
		start_ticket_channel.name.as_str() == "start_ticket_channel",
		"The only option for `/settings start_ticket_channel set` should be `start_ticket_channel`"
	);

	let original_channel = guild.get_start_ticket_channel();
	let original_message = guild.get_start_ticket_message_id();

	let CommandOptionValue::Channel(start_ticket_channel) = start_ticket_channel.value else {
		bail!("Command data is malformed; expected `start_ticket_channel` option of `/settings start_ticket_channel set` to be a channel");
	};

	if let (Some(original_channel), Some(original_message)) = (original_channel, original_message) {
		http_client
			.delete_message(original_channel, original_message)
			.reason("Message channel was changed")
			.await
			.into_diagnostic()?;
	}

	let create_ticket_button = new_ticket_button(guild_id);
	let new_message = http_client
		.create_message(start_ticket_channel)
		.content(&guild.start_ticket_message)
		.components(&[create_ticket_button])
		.await
		.into_diagnostic()?;
	let new_message = new_message.model().await.into_diagnostic()?;
	let new_message_id = new_message.id;
	let db_message_id = database_id_from_discord_id(new_message_id.get());

	let db_channel_id = database_id_from_discord_id(start_ticket_channel.get());

	let db_result = diesel::update(guilds::table)
		.filter(guilds::guild_id.eq(guild.guild_id))
		.set((
			guilds::start_ticket_channel.eq(Some(db_channel_id)),
			guilds::start_ticket_message_id.eq(Some(db_message_id)),
		))
		.execute(db_connection);
	let interaction_client = http_client.interaction(application_id);
	match db_result {
		Ok(_) => {
			let response = InteractionResponseDataBuilder::new()
				.content(format!(
					"Updated the start ticket channel to {}.",
					start_ticket_channel.mention()
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
		}
		Err(error) => {
			tracing::error!(source = ?error, "Failed to update the start ticket channel for a server");
			let response = InteractionResponseDataBuilder::new()
				.content("An internal error caused the update to fail.")
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
		}
	}

	Ok(())
}

async fn unset_ticket_channel(
	interaction: &InteractionCreate,
	guild: &Guild,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection: &mut PgConnection,
) -> miette::Result<()> {
	let original_channel = guild.get_start_ticket_channel();
	let original_message = guild.get_start_ticket_message_id();

	let interaction_client = http_client.interaction(application_id);
	let response = match original_channel {
		Some(original_channel) => {
			let no_id: Option<i64> = None;
			let db_result = diesel::update(guilds::table)
				.filter(guilds::guild_id.eq(guild.guild_id))
				.set((
					guilds::start_ticket_channel.eq(no_id),
					guilds::start_ticket_message_id.eq(no_id),
				))
				.execute(db_connection);
			match db_result {
				Ok(_) => {
					if let Some(original_message) = original_message {
						http_client
							.delete_message(original_channel, original_message)
							.reason("Removing start ticket message")
							.await
							.into_diagnostic()?;
					}
					InteractionResponseDataBuilder::new().content("The start ticket channel has been unset. Your server will no longer use this feature until you set it again.").build()
				}
				Err(error) => {
					tracing::error!(source = ?error, "Failed to remove the start ticket channel for a server");
					InteractionResponseDataBuilder::new()
						.content("An internal error occurred, so the start ticket channel couldn't be removed.")
						.flags(MessageFlags::EPHEMERAL)
						.build()
				}
			}
		}
		None => InteractionResponseDataBuilder::new()
			.content("Your server didn't have this channel set up, so the setting value remains unset.")
			.flags(MessageFlags::EPHEMERAL)
			.build(),
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
