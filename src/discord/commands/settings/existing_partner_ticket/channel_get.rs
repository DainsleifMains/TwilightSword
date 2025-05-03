// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::Guild;
use miette::IntoDiagnostic;
use twilight_http::client::Client;
use twilight_mention::fmt::Mention;
use twilight_model::channel::message::AllowedMentions;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use twilight_util::builder::InteractionResponseDataBuilder;

/// Gets the ticket channel associated with the existing partner ticket type
pub async fn execute(
	interaction: &InteractionCreate,
	guild: &Guild,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
) -> miette::Result<()> {
	let channel = guild.get_existing_partner_ticket_channel();

	let interaction_client = http_client.interaction(application_id);
	let response_content = match channel {
		Some(channel) => format!(
			"The existing partner ticket channel is set up as {}.",
			channel.mention()
		),
		None => String::from("No existing partner ticket channel is set."),
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
