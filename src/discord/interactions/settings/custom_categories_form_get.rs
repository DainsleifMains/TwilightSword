// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::SELECT_SESSION_EXPIRED_TEXT;
use crate::discord::state::settings::custom_categories_form_get::{
	CustomCategoryFormRetrievals, custom_category_form_retrieval_components,
};
use crate::model::{CustomCategory, Form};
use crate::schema::{custom_categories, forms};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{IntoDiagnostic, bail};
use std::sync::Arc;
use tokio::sync::RwLock;
use twilight_http::client::Client;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use twilight_util::builder::InteractionResponseDataBuilder;
use type_map::concurrent::TypeMap;

pub async fn route_custom_categories_form_get_interaction(
	interaction: &InteractionCreate,
	interaction_data: &MessageComponentInteractionData,
	custom_id_path: &[String],
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let Some(action) = custom_id_path.get(3) else {
		bail!(
			"Invalid custom_id path missing action: {:?}\n{:?}",
			custom_id_path,
			interaction_data
		);
	};

	match action.as_str() {
		"category" => {
			select_category(
				interaction,
				interaction_data,
				custom_id_path,
				http_client,
				application_id,
				db_connection_pool,
				bot_state,
			)
			.await
		}
		_ => bail!(
			"Invalid action in custom_id path for custom category form retrieval: {}\n{:?}",
			action,
			interaction_data
		),
	}
}

async fn select_category(
	interaction: &InteractionCreate,
	interaction_data: &MessageComponentInteractionData,
	custom_id_path: &[String],
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let Some(session_id) = custom_id_path.get(2) else {
		bail!(
			"Invalid custom_id path missing session ID: {:?}\n{:?}",
			custom_id_path,
			interaction_data
		);
	};

	let Some(selected_value) = interaction_data.values.first() else {
		bail!("Expected selection for select menu value: {:?}", interaction_data);
	};

	let interaction_client = http_client.interaction(application_id);

	let mut state = bot_state.write().await;

	let Some(data_sessions) = state.get_mut::<CustomCategoryFormRetrievals>() else {
		drop(state);
		let response = InteractionResponseDataBuilder::new()
			.content(SELECT_SESSION_EXPIRED_TEXT)
			.components(Vec::new())
			.build();
		let response = InteractionResponse {
			kind: InteractionResponseType::UpdateMessage,
			data: Some(response),
		};
		interaction_client
			.create_response(interaction.id, &interaction.token, &response)
			.await
			.into_diagnostic()?;
		return Ok(());
	};

	if let Some(new_page) = selected_value.strip_prefix('>') {
		let new_page: usize = new_page.parse().into_diagnostic()?;
		let Some(session_data) = data_sessions.sessions.get_mut(session_id) else {
			drop(state);
			let response = InteractionResponseDataBuilder::new()
				.content(SELECT_SESSION_EXPIRED_TEXT)
				.components(Vec::new())
				.build();
			let response = InteractionResponse {
				kind: InteractionResponseType::UpdateMessage,
				data: Some(response),
			};
			interaction_client
				.create_response(interaction.id, &interaction.token, &response)
				.await
				.into_diagnostic()?;
			return Ok(());
		};
		session_data.current_page = new_page;

		let components =
			custom_category_form_retrieval_components(session_id, &session_data.all_categories, None, new_page);
		let response = InteractionResponseDataBuilder::new().components(components).build();
		let response = InteractionResponse {
			kind: InteractionResponseType::UpdateMessage,
			data: Some(response),
		};
		interaction_client
			.create_response(interaction.id, &interaction.token, &response)
			.await
			.into_diagnostic()?;
		return Ok(());
	}

	data_sessions.sessions.remove(session_id);
	drop(state);

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;
	let custom_category: QueryResult<CustomCategory> =
		custom_categories::table.find(selected_value).first(&mut db_connection);

	let response_content = match custom_category {
		Ok(category) => {
			let category_name = category.name.replace("`", "\\`");
			match &category.form {
				Some(form_id) => {
					let form: QueryResult<Form> = forms::table.find(form_id).first(&mut db_connection);
					match form {
						Ok(form_data) => {
							let form_name = form_data.title.replace("`", "\\`");
							format!("The form for `{}` is `{}`.", category_name, form_name)
						}
						Err(error) => {
							tracing::error!(source = ?error, "Failed to get the form for a custom category");
							String::from("An internal error prevented getting the form name.")
						}
					}
				}
				None => format!("There is no form set for `{}`.", category_name),
			}
		}
		Err(error) => {
			tracing::error!(source = ?error, "Failed to get a custom category");
			String::from("An internal error prevented getting the form name.")
		}
	};
	let response = InteractionResponseDataBuilder::new()
		.content(response_content)
		.components(Vec::new())
		.build();
	let response = InteractionResponse {
		kind: InteractionResponseType::UpdateMessage,
		data: Some(response),
	};
	interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.await
		.into_diagnostic()?;

	Ok(())
}
