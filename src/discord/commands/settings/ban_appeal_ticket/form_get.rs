// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::Guild;
use crate::schema::forms;
use diesel::prelude::*;
use miette::IntoDiagnostic;
use twilight_http::client::Client;
use twilight_model::channel::message::AllowedMentions;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use twilight_util::builder::InteractionResponseDataBuilder;

/// Gets the form associated with the ban appeal ticket type
pub async fn execute(
	interaction: &InteractionCreate,
	guild: &Guild,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection: &mut PgConnection,
) -> miette::Result<()> {
	let form_id = guild.ban_appeal_ticket_form.as_ref();

	let interaction_client = http_client.interaction(application_id);
	let response_content = match form_id {
		Some(form_id) => {
			let mut form_name: String = forms::table
				.find(form_id)
				.select(forms::title)
				.first(db_connection)
				.into_diagnostic()?;
			form_name = form_name.replace("`", "\\`");
			format!("Ban appeals ticket use the form `{}`.", form_name)
		}
		None => String::from("No ban appeal form is set."),
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
