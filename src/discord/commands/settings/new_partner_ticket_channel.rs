// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::responses::NOT_SET_UP_FOR_GUILD;
use crate::model::{database_id_from_discord_id, Guild};
use crate::schema::guilds;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{bail, ensure, IntoDiagnostic};
use std::sync::Arc;
use twilight_http::client::Client;
use twilight_mention::fmt::Mention;
use twilight_model::application::command::CommandOption;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::message::{AllowedMentions, MessageFlags};
use twilight_model::channel::ChannelType;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use twilight_util::builder::command::{ChannelBuilder, SubCommandBuilder, SubCommandGroupBuilder};
use twilight_util::builder::InteractionResponseDataBuilder;

pub fn subcommand_definition() -> CommandOption {
	let channel_option = ChannelBuilder::new(
		"new_partner_ticket_channel",
		"The channel in which new partner tickets are posted",
	)
	.channel_types([ChannelType::GuildForum])
	.required(true)
	.build();

	let get = SubCommandBuilder::new("get", "Gets the new partner ticket channel");
	let set = SubCommandBuilder::new("set", "Sets the new partner ticket channel").option(channel_option);
	let unset = SubCommandBuilder::new("unset", "Removes the new partner ticket channel");

	SubCommandGroupBuilder::new(
		"new_partner_ticket_channel",
		"Manages the channel to which new partner tickets are posted",
	)
	.subcommands([get, set, unset])
	.build()
}

pub async fn handle_subcommand(
	interaction: &InteractionCreate,
	subcommand_value: &CommandOptionValue,
	http_client: Arc<Client>,
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
			tracing::error!(source = ?error, "Failed to retrieve guild for getting or updating new partner ticket channel");
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
		bail!("Command data is malformed; expected `/settings new_partner_ticket_channel` to get a subcommand group value");
	};
	let Some(value) = value_data.first() else {
		bail!("Command data is malformed; expected `/settings new_partner_ticket_channel` to have a subcommand");
	};
	match value.name.as_str() {
		"get" => get_ticket_channel(interaction, &guild, http_client, application_id).await,
		"set" => {
			set_ticket_channel(
				interaction,
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
			"Unknown settings new_partner_ticket_channel subcommand encountered: {}\n{:?}",
			value.name,
			subcommand_value
		),
	}
}

async fn get_ticket_channel(
	interaction: &InteractionCreate,
	guild: &Guild,
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
) -> miette::Result<()> {
	let channel = guild.get_new_partner_ticket_channel();

	let interaction_client = http_client.interaction(application_id);
	let response_content = match channel {
		Some(channel) => format!("The new partner ticket channel is set up as {}.", channel.mention()),
		None => String::from("No new partner ticket channel is set."),
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
	guild: &Guild,
	subcommand_value: &CommandOptionValue,
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection: &mut PgConnection,
) -> miette::Result<()> {
	let CommandOptionValue::SubCommand(values) = subcommand_value else {
		bail!("Command data is malformed; expected `/settings new_partner_ticket_channel set` to get subcommand data");
	};
	let Some(new_partner_ticket_channel) = values.first() else {
		bail!("Command data is malformed; expected `/settings new_partner_ticket_channel set` to have required option `new_partner_ticket_channel`");
	};
	ensure!(
		new_partner_ticket_channel.name.as_str() == "new_partner_ticket_channel",
		"The only option for `/settings new_partner_ticket_channel set` should be `new_partner_ticket_channel`"
	);

	let CommandOptionValue::Channel(new_partner_ticket_channel) = new_partner_ticket_channel.value else {
		bail!("Command data is malformed; expected `new_partner_ticket_channel` option of `/settings new_partner_ticket_channel set` to be a channel");
	};

	let db_channel_id = database_id_from_discord_id(new_partner_ticket_channel.get());

	let db_result = diesel::update(guilds::table)
		.filter(guilds::guild_id.eq(guild.guild_id))
		.set(guilds::new_partner_ticket_channel.eq(Some(db_channel_id)))
		.execute(db_connection);
	let interaction_client = http_client.interaction(application_id);
	match db_result {
		Ok(_) => {
			let response = InteractionResponseDataBuilder::new()
				.content(format!(
					"Updated the new partner ticket channel to {}.",
					new_partner_ticket_channel.mention()
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
			tracing::error!(source = ?error, "Failed to update the new partner ticket channel for a server");
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
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection: &mut PgConnection,
) -> miette::Result<()> {
	let original_channel = guild.get_new_partner_ticket_channel();
	let interaction_client = http_client.interaction(application_id);
	let response = match original_channel {
		Some(_) => {
			let no_id: Option<i64> = None;
			let db_result = diesel::update(guilds::table)
				.filter(guilds::guild_id.eq(guild.guild_id))
				.set(guilds::new_partner_ticket_channel.eq(no_id))
				.execute(db_connection);
			match db_result {
				Ok(_) => InteractionResponseDataBuilder::new()
					.content("The new partner ticket channel has been unset.")
					.build(),
				Err(error) => {
					tracing::error!(source = ?error, "Failed to remove the new partner ticket channel for a server");
					InteractionResponseDataBuilder::new()
						.content("An internal error occurred, so the new partner ticket channel couldn't be unset.")
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