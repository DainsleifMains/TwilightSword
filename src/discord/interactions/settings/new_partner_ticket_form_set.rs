// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::FORM_SESSION_EXPIRED_TEXT;
use crate::discord::state::settings::new_partner_ticket_form_set::{
	NewPartnerFormAssociations, new_partner_form_association_components,
};
use crate::model::{Form, database_id_from_discord_id};
use crate::schema::{forms, guilds};
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

pub async fn route_new_partner_ticket_form_set_interaction(
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
		"form" => {
			selected_form(
				interaction,
				interaction_data,
				custom_id_path,
				http_client,
				application_id,
				bot_state,
			)
			.await
		}
		"submit" => {
			submit_form_selection(
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
			"Invalid action in custom_id path for new partner ticket form selection: {}\n{:?}",
			action,
			interaction_data
		),
	}
}

async fn selected_form(
	interaction: &InteractionCreate,
	interaction_data: &MessageComponentInteractionData,
	custom_id_path: &[String],
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
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

	let Some(form_sessions) = state.get_mut::<NewPartnerFormAssociations>() else {
		let response = InteractionResponseDataBuilder::new()
			.content(FORM_SESSION_EXPIRED_TEXT)
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

	let Some(session_data) = form_sessions.sessions.get_mut(session_id) else {
		let response = InteractionResponseDataBuilder::new()
			.content(FORM_SESSION_EXPIRED_TEXT)
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
		session_data.current_page = new_page;
	} else {
		session_data.selected_form_id = Some(selected_value.clone());
	}

	let new_components = new_partner_form_association_components(
		session_id,
		&session_data.all_forms,
		session_data.selected_form_id.as_ref(),
		session_data.current_page,
	);
	let updated_message = InteractionResponseDataBuilder::new().components(new_components).build();
	let response = InteractionResponse {
		kind: InteractionResponseType::UpdateMessage,
		data: Some(updated_message),
	};
	interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.await
		.into_diagnostic()?;

	Ok(())
}

async fn submit_form_selection(
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

	let interaction_client = http_client.interaction(application_id);

	let session_data = {
		let mut state = bot_state.write().await;
		let Some(form_sessions) = state.get_mut::<NewPartnerFormAssociations>() else {
			let response = InteractionResponseDataBuilder::new()
				.content(FORM_SESSION_EXPIRED_TEXT)
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
		let Some(session_data) = form_sessions.sessions.remove(session_id) else {
			let response = InteractionResponseDataBuilder::new()
				.content(FORM_SESSION_EXPIRED_TEXT)
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
		session_data
	};

	let Some(selected_form_id) = session_data.selected_form_id else {
		let response = InteractionResponseDataBuilder::new()
			.content("No form was selected.")
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

	let db_guild_id = database_id_from_discord_id(session_data.guild_id.get());
	let mut db_connection = db_connection_pool.get().into_diagnostic()?;

	let db_result = diesel::update(guilds::table)
		.filter(guilds::guild_id.eq(db_guild_id))
		.set(guilds::new_partner_ticket_form.eq(&selected_form_id))
		.execute(&mut db_connection);

	let response = match db_result {
		Ok(_) => {
			let form_data: Form = forms::table
				.find(&selected_form_id)
				.first(&mut db_connection)
				.into_diagnostic()?;
			let form_name = form_data.title.replace("`", "`\\`");
			format!("The form for new partner tickets has been updated to `{}`.", form_name)
		}
		Err(error) => {
			tracing::error!(source = ?error, "Failed to update new partner ticket form");
			String::from("An error occurred updating the new partner ticket form.")
		}
	};
	let response = InteractionResponseDataBuilder::new()
		.content(response)
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
