// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::state::settings::form_set::{FormAssociationData, FormAssociations};
use crate::model::{Form, Guild};
use crate::schema::forms;
use diesel::prelude::*;
use miette::IntoDiagnostic;
use std::sync::Arc;
use tokio::sync::RwLock;
use twilight_http::client::Client;
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::message::component::{
	ActionRow, Button, ButtonStyle, Component, SelectMenu, SelectMenuOption, SelectMenuType,
};
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

	let mut select_options = Vec::with_capacity(25);
	let form_component = |form: &Form| SelectMenuOption {
		default: false,
		description: None,
		emoji: None,
		label: form.title.clone(),
		value: form.id.clone(),
	};
	if guild_forms.len() > 25 {
		for form in guild_forms.iter().take(23) {
			select_options.push(form_component(form));
		}
		select_options.push(SelectMenuOption {
			default: false,
			description: Some(String::from("See next page of forms")),
			emoji: None,
			label: String::from("Next Page"),
			value: String::from(">1"),
		});
	} else {
		for form in guild_forms.iter() {
			select_options.push(form_component(form));
		}
	}

	let session_id = cuid2::create_id();

	let select_menu = SelectMenu {
		channel_types: None,
		custom_id: format!("{}/form", session_id),
		default_values: None,
		disabled: false,
		kind: SelectMenuType::Text,
		max_values: None,
		min_values: None,
		options: Some(select_options),
		placeholder: Some(String::from("Form")),
	};
	let select_component = Component::SelectMenu(select_menu);
	let select_row = Component::ActionRow(ActionRow {
		components: vec![select_component],
	});

	let button_component = Component::Button(Button {
		custom_id: Some(format!("{}/confirm", session_id)),
		disabled: true,
		emoji: None,
		label: Some(String::from("Select Form")),
		style: ButtonStyle::Primary,
		url: None,
		sku_id: None,
	});
	let button_row = Component::ActionRow(ActionRow {
		components: vec![button_component],
	});

	let components = vec![select_row, button_row];

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
	};

	let mut state = bot_state.write().await;
	let all_sessions = state.entry().or_insert_with(FormAssociations::default);
	all_sessions.sessions.insert(session_id, session_data);

	Ok(())
}
