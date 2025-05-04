// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::state::settings::ban_appeal_ticket_form_set::{
	FormAssociationData, FormAssociations, form_association_components,
};
use crate::model::{Form, Guild};
use crate::schema::forms;
use diesel::prelude::*;
use miette::IntoDiagnostic;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};
use twilight_http::client::Client;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::{ApplicationMarker, GuildMarker};
use twilight_util::builder::InteractionResponseDataBuilder;
use type_map::concurrent::TypeMap;

/// Sets the form to use for ban appeal tickets
pub async fn execute(
	interaction: &InteractionCreate,
	guild_id: Id<GuildMarker>,
	guild: &Guild,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection: &mut PgConnection,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let guild_forms: Vec<Form> = forms::table
		.filter(forms::guild.eq(guild.guild_id))
		.order(forms::title.asc())
		.load(db_connection)
		.into_diagnostic()?;

	let interaction_client = http_client.interaction(application_id);

	if guild_forms.is_empty() {
		let response = InteractionResponseDataBuilder::new()
			.content("There are no forms set up for this server.")
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

	let session_id = cuid2::create_id();

	let components = form_association_components(&session_id, &guild_forms, None, 0);

	let response = InteractionResponseDataBuilder::new().components(components).build();
	let response = InteractionResponse {
		kind: InteractionResponseType::ChannelMessageWithSource,
		data: Some(response),
	};
	interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.await
		.into_diagnostic()?;

	let session_data = FormAssociationData {
		guild_id,
		all_forms: guild_forms,
		selected_form_id: None,
		current_page: 0,
	};

	{
		let mut state = bot_state.write().await;
		let all_sessions = state.entry().or_insert_with(FormAssociations::default);
		all_sessions.sessions.insert(session_id.clone(), session_data);
	}

	tokio::spawn(expire_session(bot_state, session_id));

	Ok(())
}

async fn expire_session(bot_state: Arc<RwLock<TypeMap>>, session_id: String) {
	sleep(Duration::from_secs(3600)).await;
	let mut state = bot_state.write().await;
	let all_sessions = state.get_mut::<FormAssociations>();
	if let Some(all_sessions) = all_sessions {
		all_sessions.sessions.remove(&session_id);
	}
}
