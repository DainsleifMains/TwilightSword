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

/// Unsets the form associated with new partner tickets
pub async fn execute(
	interaction: &InteractionCreate,
	guild: &Guild,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection: &mut PgConnection,
) -> miette::Result<()> {
	let interaction_client = http_client.interaction(application_id);
	let response = match guild.new_partner_ticket_form.as_ref() {
		Some(_) => {
			let no_form: Option<String> = None;
			let db_result = diesel::update(guilds::table)
				.filter(guilds::guild_id.eq(&guild.guild_id))
				.set(guilds::new_partner_ticket_form.eq(no_form))
				.execute(db_connection);
			match db_result {
				Ok(_) => InteractionResponseDataBuilder::new()
					.content("The new partner ticket form has been unset.")
					.build(),
				Err(error) => {
					tracing::error!(source = ?error, "Failed to remove the new partner ticket form for a server");
					InteractionResponseDataBuilder::new()
						.content("An internal error occurred, so the new partner ticket form couldn't be unset.")
						.flags(MessageFlags::EPHEMERAL)
						.build()
				}
			}
		}
		None => InteractionResponseDataBuilder::new()
			.content("There is no new partner ticket form.")
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
