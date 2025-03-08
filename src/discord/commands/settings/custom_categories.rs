// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::utils::permissions::{
	channel_permissions, ticket_channel_missing_permissions_message, ticket_channel_permissions,
};
use crate::discord::utils::setup::NOT_SET_UP_FOR_GUILD;
use crate::model::{CustomCategory, Guild, database_id_from_discord_id};
use crate::schema::{custom_categories, guilds};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{IntoDiagnostic, bail};
use twilight_http::client::Client;
use twilight_mention::fmt::Mention;
use twilight_model::application::command::CommandOption;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::ChannelType;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use twilight_util::builder::InteractionResponseDataBuilder;
use twilight_util::builder::command::{ChannelBuilder, StringBuilder, SubCommandBuilder, SubCommandGroupBuilder};

pub fn subcommand_definition() -> CommandOption {
	let name_option = StringBuilder::new("name", "Name of the new category")
		.max_length(100)
		.required(true)
		.build();
	let channel_option = ChannelBuilder::new("channel", "The channel to which tickets in this category are posted")
		.channel_types([ChannelType::GuildForum])
		.required(true)
		.build();

	let create = SubCommandBuilder::new("create", "Creates a new custom category")
		.option(name_option)
		.option(channel_option);

	SubCommandGroupBuilder::new("custom_categories", "Manages custom ticket categories")
		.subcommands([create])
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
		Err(error) => {
			tracing::error!(source = ?error, "Failed to retrieve guild for managing custom category settings");
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
		bail!("Command data is malformed; expected `/settings custom_categories` to get a subcommand group value");
	};
	let Some(value) = value_data.first() else {
		bail!("Command data is malformed; expected `/settings custom_categories` to have a subcommand");
	};
	match value.name.as_str() {
		"create" => {
			create_category(
				interaction,
				&value.value,
				&guild,
				http_client,
				application_id,
				&mut db_connection,
			)
			.await
		}
		_ => bail!(
			"Unknown settings custom_categories subcommand encountered: {}\n{:?}",
			value.name,
			subcommand_value
		),
	}
}

async fn create_category(
	interaction: &InteractionCreate,
	subcommand_value: &CommandOptionValue,
	guild: &Guild,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection: &mut PgConnection,
) -> miette::Result<()> {
	let CommandOptionValue::SubCommand(values) = subcommand_value else {
		bail!("Command data is malformed; expected `/settings custom_categories create` to get subcommand data");
	};

	let mut name: Option<&CommandOptionValue> = None;
	let mut channel: Option<&CommandOptionValue> = None;
	for value in values.iter() {
		match value.name.as_str() {
			"name" => name = Some(&value.value),
			"channel" => channel = Some(&value.value),
			_ => (),
		}
	}

	let Some(name) = name else {
		bail!("Required option `name` for `/settings custom_categories create` was missing");
	};
	let Some(channel) = channel else {
		bail!("Required option `channel` for `/settings custom_categories create` was missing");
	};

	let CommandOptionValue::String(name) = name else {
		bail!(
			"Command data is malformed; expected `name` option of `/setting custom_categories create` to be a string"
		);
	};
	let CommandOptionValue::Channel(channel) = *channel else {
		bail!(
			"Command data is malformed; expected `channel` option of `/settings custom_categories create` to be a channel"
		);
	};

	let interaction_client = http_client.interaction(application_id);
	let existing_category: QueryResult<Option<CustomCategory>> = custom_categories::table
		.filter(
			custom_categories::guild
				.eq(guild.guild_id)
				.and(custom_categories::name.eq(&name))
				.and(custom_categories::active.eq(true)),
		)
		.first(db_connection)
		.optional();
	match existing_category {
		Ok(Some(_)) => {
			let response = InteractionResponseDataBuilder::new()
				.content("An existing category with the same exact name already exists for your server.")
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
		Ok(None) => (),
		Err(error) => {
			tracing::error!(source = ?error, "Failed to check for duplicate custom category during creation");
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
	}

	let guild_id = guild.get_guild_id();
	let permissions_in_channel = channel_permissions(guild_id, channel, http_client).await?;
	if !permissions_in_channel.contains(ticket_channel_permissions()) {
		let response_content = ticket_channel_missing_permissions_message(channel.mention());
		let response = InteractionResponseDataBuilder::new()
			.content(response_content)
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

	let db_channel_id = database_id_from_discord_id(channel.get());

	let new_category = CustomCategory {
		id: cuid2::create_id(),
		guild: guild.guild_id,
		name: name.clone(),
		channel: db_channel_id,
		form: None,
		active: true,
	};
	let create_result = diesel::insert_into(custom_categories::table)
		.values(new_category)
		.execute(db_connection);

	let response = match create_result {
		Ok(_) => InteractionResponseDataBuilder::new()
			.content(format!(
				"Created new ticket category `{}` in channel {}.",
				name,
				channel.mention()
			))
			.build(),
		Err(error) => {
			tracing::error!(source = ?error, "Failed to add new custom category");
			InteractionResponseDataBuilder::new()
				.content("An internal error occurred creating the new category.")
				.flags(MessageFlags::EPHEMERAL)
				.build()
		}
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
