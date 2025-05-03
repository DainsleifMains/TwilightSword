// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::Guild;
use crate::schema::guilds;
use diesel::prelude::*;
use miette::IntoDiagnostic;
use twilight_http::client::Client;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use twilight_util::builder::InteractionResponseDataBuilder;

/// Removes the channel associated with the existing partner ticket type
pub async fn execute(
	interaction: &InteractionCreate,
	guild: &Guild,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection: &mut PgConnection,
) -> miette::Result<()> {
	let original_channel = guild.get_existing_partner_ticket_channel();
	let interaction_client = http_client.interaction(application_id);
	let response = match original_channel {
		Some(_) => {
			let no_id: Option<i64> = None;
			let db_result = diesel::update(guilds::table)
				.filter(guilds::guild_id.eq(guild.guild_id))
				.set(guilds::existing_partner_ticket_channel.eq(no_id))
				.execute(db_connection);
			match db_result {
				Ok(_) => InteractionResponseDataBuilder::new()
					.content("The existing partner ticket channel has been unset.")
					.build(),
				Err(error) => {
					tracing::error!(source = ?error, "Failed to remove the existing partner ticket channel for a server");
					InteractionResponseDataBuilder::new()
						.content(
							"An internal error occurred, so the existing partner ticket channel couldn't be unset.",
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
