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
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use twilight_util::builder::command::{RoleBuilder, SubCommandBuilder, SubCommandGroupBuilder};
use twilight_util::builder::InteractionResponseDataBuilder;

pub fn subcommand_definition() -> CommandOption {
	let admin_option = RoleBuilder::new("admin_role", "The role assigned to all administrators")
		.required(true)
		.build();

	let get_subcommand = SubCommandBuilder::new(
		"get",
		"Gets the setting value for the role assigned to all administrators",
	);
	let set_subcommand =
		SubCommandBuilder::new("set", "Sets the role assigned to all administrators").option(admin_option);

	SubCommandGroupBuilder::new("admin_role", "Manages the admin role setting")
		.subcommands([get_subcommand, set_subcommand])
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
			tracing::error!(source = ?error, "Failed to retrieve guild for getting or updating admin role");
			let response = InteractionResponseDataBuilder::new()
				.content("An internal error occurred handling this command.")
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
		bail!("Command data is malformed; expected `/settings admin_role` to get a subcommand group value");
	};
	let Some(value) = value_data.first() else {
		bail!("Command data is malformed; expected `/settings admin_role` to have a subcommand");
	};
	match value.name.as_str() {
		"get" => get_admin_role(interaction, &guild, http_client, application_id).await,
		"set" => {
			set_admin_role(
				interaction,
				&guild,
				&value.value,
				http_client,
				application_id,
				db_connection_pool,
			)
			.await
		}
		_ => bail!(
			"Unknown settings admin_role subcommand encountered: {}\n{:?}",
			value.name,
			subcommand_value
		),
	}
}

async fn get_admin_role(
	interaction: &InteractionCreate,
	guild: &Guild,
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
) -> miette::Result<()> {
	let role = guild.get_admin_role();

	let interaction_client = http_client.interaction(application_id);
	let response = InteractionResponseDataBuilder::new()
		.content(format!("The admin role is set up as {}.", role.mention()))
		.allowed_mentions(AllowedMentions::default())
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

	Ok(())
}

async fn set_admin_role(
	interaction: &InteractionCreate,
	guild: &Guild,
	subcommand_value: &CommandOptionValue,
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> miette::Result<()> {
	let CommandOptionValue::SubCommand(values) = subcommand_value else {
		bail!("Command data is malformed; expected `/settings admin_role set` to get subcommand data");
	};
	let Some(admin_role) = values.first() else {
		bail!("Command data is malformed; expected `/settings admin_role set` to have required option `admin_role`");
	};
	ensure!(
		admin_role.name.as_str() == "admin_role",
		"The only option for `/settings admin_role set` should be `admin_role`"
	);

	let CommandOptionValue::Role(new_admin_role) = admin_role.value else {
		bail!("Command data is malformed; expected `admin_role` option of `/settings admin_role set` to be a role");
	};

	let db_role_id = database_id_from_discord_id(new_admin_role.get());

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;
	let db_result = diesel::update(guilds::table)
		.filter(guilds::guild_id.eq(guild.guild_id))
		.set(guilds::admin_role.eq(db_role_id))
		.execute(&mut db_connection);
	let interaction_client = http_client.interaction(application_id);
	match db_result {
		Ok(_) => {
			let response = InteractionResponseDataBuilder::new()
				.content(format!("Updated the admin role to {}.", new_admin_role.mention()))
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
			tracing::error!(source = ?error, "Failed to update the admin role for a server");
			let response = InteractionResponseDataBuilder::new()
				.content("An internal error caused the update to fail.")
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
