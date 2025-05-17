// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::state::settings::custom_categories_form_unset::{
	CustomCategoryFormRemovalData, CustomCategoryFormRemovals, custom_category_form_removal_components,
};
use crate::model::{CustomCategory, Guild};
use crate::schema::custom_categories;
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
use twilight_model::id::marker::ApplicationMarker;
use twilight_util::builder::InteractionResponseDataBuilder;
use type_map::concurrent::TypeMap;

pub async fn execute(
	interaction: &InteractionCreate,
	guild: &Guild,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection: &mut PgConnection,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let interaction_client = http_client.interaction(application_id);

	let all_categories: QueryResult<Vec<CustomCategory>> = custom_categories::table
		.filter(custom_categories::guild.eq(guild.guild_id))
		.order(custom_categories::name.asc())
		.load(db_connection);
	let all_categories = match all_categories {
		Ok(categories) => categories,
		Err(error) => {
			tracing::error!(source = ?error, "Failed to get custom categories for a guild");
			let response = InteractionResponseDataBuilder::new()
				.content("An internal error prevented retrieval of necessary data.")
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

	if all_categories.is_empty() {
		let response = InteractionResponseDataBuilder::new()
			.content("This server has no custom categories set up.")
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

	let components = custom_category_form_removal_components(&session_id, &all_categories, None, 0);
	let response = InteractionResponseDataBuilder::new().components(components).build();
	let response = InteractionResponse {
		kind: InteractionResponseType::ChannelMessageWithSource,
		data: Some(response),
	};
	interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.await
		.into_diagnostic()?;

	let session_data = CustomCategoryFormRemovalData {
		all_categories,
		selected_id: None,
		current_page: 0,
	};
	{
		let mut state = bot_state.write().await;
		let data_sessions = state.entry().or_insert_with(CustomCategoryFormRemovals::default);
		data_sessions.sessions.insert(session_id.clone(), session_data);
	}

	tokio::spawn(expire_session(bot_state, session_id));

	Ok(())
}

async fn expire_session(bot_state: Arc<RwLock<TypeMap>>, session_id: String) {
	sleep(Duration::from_secs(3600)).await;
	let mut state = bot_state.write().await;
	if let Some(data_sessions) = state.get_mut::<CustomCategoryFormRemovals>() {
		data_sessions.sessions.remove(&session_id);
	}
}
