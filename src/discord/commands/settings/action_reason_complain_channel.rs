// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::utils::permissions::channel_permissions;
use crate::discord::utils::setup::NOT_SET_UP_FOR_GUILD;
use crate::model::{Guild, database_id_from_discord_id};
use crate::schema::guilds;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{IntoDiagnostic, bail, ensure};
use twilight_http::client::Client;
use twilight_mention::fmt::Mention;
use twilight_model::application::command::CommandOption;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::ChannelType;
use twilight_model::channel::message::{AllowedMentions, MessageFlags};
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::{ApplicationMarker, GuildMarker};
use twilight_util::builder::InteractionResponseDataBuilder;
use twilight_util::builder::command::{ChannelBuilder, SubCommandBuilder, SubCommandGroupBuilder};

pub fn subcommand_definition() -> CommandOption {
	let channel_option = ChannelBuilder::new(
		"action_reason_complain_channel",
		"The channel to which the bot posts complaints about missing action reasons",
	)
	.channel_types([ChannelType::GuildText])
	.required(true)
	.build();

	let get = SubCommandBuilder::new("get", "Gets the action reason complain channel");
	let set = SubCommandBuilder::new("set", "Sets the action reason complain channel").option(channel_option);
	let unset = SubCommandBuilder::new("unset", "Removes the action reason complain channel");

	SubCommandGroupBuilder::new(
		"action_reason_complain_channel",
		"Manages the channel to which complaints about missing action reasons are sent",
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
			tracing::error!(source = ?error, "Failed to retrieve guild for getting or updating action reason complain channel");
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
		bail!(
			"Command data is malformed; expected `/settings action_reason_complain_channel` to get a subcommand group value"
		);
	};
	let Some(value) = value_data.first() else {
		bail!("Command data is malformed; expected `/settings action_reason_complain_channel to have a subcommand");
	};
	match value.name.as_str() {
		"get" => get_complain_channel(interaction, &guild, http_client, application_id).await,
		"set" => {
			set_complain_channel(
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
		"unset" => unset_complain_channel(interaction, &guild, http_client, application_id, &mut db_connection).await,
		_ => bail!(
			"Unknown settings action_reason_complain_channel subcommand encountered: {}\n{:?}",
			value.name,
			subcommand_value
		),
	}
}

async fn get_complain_channel(
	interaction: &InteractionCreate,
	guild: &Guild,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
) -> miette::Result<()> {
	let channel = guild.get_action_reason_complain_channel();

	let interaction_client = http_client.interaction(application_id);
	let response_content = match channel {
		Some(channel) => format!(
			"The action reason complaint channel is set up as {}.",
			channel.mention()
		),
		None => String::from("No action reason complaint channel is set."),
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

async fn set_complain_channel(
	interaction: &InteractionCreate,
	guild_id: Id<GuildMarker>,
	guild: &Guild,
	subcommand_value: &CommandOptionValue,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection: &mut PgConnection,
) -> miette::Result<()> {
	let CommandOptionValue::SubCommand(values) = subcommand_value else {
		bail!(
			"Command data is malformed; expected `/settings action_reason_complain_channel set` to get subcommand data"
		);
	};
	let Some(action_reason_complain_channel) = values.first() else {
		bail!(
			"Command data is malformed; expected `/setings action_reason_complain_channel set` to have required option `action_reason_complain_channel`"
		);
	};
	ensure!(
		action_reason_complain_channel.name.as_str() == "action_reason_complain_channel",
		"The only option for `/settings action_reason_complain_channel set` should be `action_reason_complain_channel`"
	);

	let CommandOptionValue::Channel(action_reason_complain_channel) = action_reason_complain_channel.value else {
		bail!(
			"Command data is malformed; expected `action_reason_complain_channel` option of `/settings action_reason_complain_channel set` to be a channel"
		);
	};

	let permissions_in_channel = channel_permissions(guild_id, action_reason_complain_channel, http_client).await?;

	let interaction_client = http_client.interaction(application_id);
	if !permissions_in_channel.contains(Permissions::SEND_MESSAGES) {
		let response_content = format!(
			"The channel {} doesn't have the necessary permissions (Send Messages) for me to post to it.",
			action_reason_complain_channel.mention()
		);
		let response = InteractionResponseDataBuilder::new().content(response_content).build();
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

	let db_channel_id = database_id_from_discord_id(action_reason_complain_channel.get());

	let db_result = diesel::update(guilds::table)
		.filter(guilds::guild_id.eq(guild.guild_id))
		.set(guilds::action_reason_complain_channel.eq(Some(db_channel_id)))
		.execute(db_connection);
	match db_result {
		Ok(_) => {
			let response = InteractionResponseDataBuilder::new()
				.content(format!(
					"Updated the action reason complaint channel to {}.",
					action_reason_complain_channel.mention()
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
			tracing::error!(source = ?error, "Failed to update the action reason complain channel for a server");
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

async fn unset_complain_channel(
	interaction: &InteractionCreate,
	guild: &Guild,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection: &mut PgConnection,
) -> miette::Result<()> {
	let original_channel = guild.get_action_reason_complain_channel();
	let interaction_client = http_client.interaction(application_id);
	let response = match original_channel {
		Some(_) => {
			let no_id: Option<i64> = None;
			let db_result = diesel::update(guilds::table)
				.filter(guilds::guild_id.eq(guild.guild_id))
				.set(guilds::action_reason_complain_channel.eq(no_id))
				.execute(db_connection);
			match db_result {
				Ok(_) => InteractionResponseDataBuilder::new()
					.content("The action reason complaint channel has been unset.")
					.build(),
				Err(error) => {
					tracing::error!(source = ?error, "Failed to remove the action reason complain channel for a server");
					InteractionResponseDataBuilder::new()
						.content(
							"An internal error occurred, so the action reason complaint channel couldn't be unset.",
						)
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
