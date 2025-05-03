// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::utils::permissions::{
	channel_permissions, ticket_channel_missing_permissions_message, ticket_channel_permissions,
};
use crate::model::{Guild, database_id_from_discord_id};
use crate::schema::guilds;
use diesel::prelude::*;
use miette::{IntoDiagnostic, bail, ensure};
use twilight_http::client::Client;
use twilight_mention::fmt::Mention;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::message::{AllowedMentions, MessageFlags};
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::{ApplicationMarker, GuildMarker};
use twilight_util::builder::InteractionResponseDataBuilder;

/// Sets the ticket channel associated with the existing partner ticket type
pub async fn execute(
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
			"Command data is malformed; expected `/settings existing_partner_ticket channel_set` to get subcommand data"
		);
	};
	let Some(existing_partner_ticket_channel) = values.first() else {
		bail!(
			"Command data is malformed; expected `/settings existing_partner_ticket channel_set` to have required option `existing_partner_ticket_channel`"
		);
	};
	ensure!(
		existing_partner_ticket_channel.name.as_str() == "existing_partner_ticket_channel",
		"The only option for `/settings existing_partner_ticket channel_set` should be `existing_partner_ticket_channel`"
	);

	let CommandOptionValue::Channel(existing_partner_ticket_channel) = existing_partner_ticket_channel.value else {
		bail!(
			"Command data is malformed; expected `existing_partner_ticket_channel` option of `/settings existing_partner_ticket channel_set` to be a channel"
		);
	};

	let permissions_in_channel = channel_permissions(guild_id, existing_partner_ticket_channel, http_client).await?;
	let interaction_client = http_client.interaction(application_id);
	if !permissions_in_channel.contains(ticket_channel_permissions()) {
		let response_content = ticket_channel_missing_permissions_message(existing_partner_ticket_channel.mention());
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

	let db_channel_id = database_id_from_discord_id(existing_partner_ticket_channel.get());

	let db_result = diesel::update(guilds::table)
		.filter(guilds::guild_id.eq(guild.guild_id))
		.set(guilds::existing_partner_ticket_channel.eq(Some(db_channel_id)))
		.execute(db_connection);
	match db_result {
		Ok(_) => {
			let response = InteractionResponseDataBuilder::new()
				.content(format!(
					"Updated the existing partner ticket channel to {}.",
					existing_partner_ticket_channel.mention()
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
			tracing::error!(source = ?error, "Failed to update the existing partner ticket channel for a server");
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
