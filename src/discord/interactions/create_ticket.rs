// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::interactions::MAX_INTERACTION_WAIT_TIME;
use crate::discord::state::create_ticket::{BuiltInCategory, CreateTicketState, CreateTicketStates};
use crate::model::{database_id_from_discord_id, CustomCategory, Guild, Ticket};
use crate::schema::{custom_categories, guilds, tickets};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{bail, IntoDiagnostic};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::sleep;
use twilight_http::client::Client;
use twilight_model::channel::message::component::{
	ActionRow, Button, ButtonStyle, Component, SelectMenu, SelectMenuOption, SelectMenuType,
};
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use twilight_util::builder::InteractionResponseDataBuilder;
use type_map::concurrent::TypeMap;

pub async fn create_ticket(
	interaction: &InteractionCreate,
	http_client: &Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let Some(guild_id) = interaction.guild_id else {
		bail!("Create Ticket button used outside of a guild");
	};

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;
	let db_guild_id = database_id_from_discord_id(guild_id.get());

	let guild: Option<Guild> = guilds::table
		.find(db_guild_id)
		.first(&mut db_connection)
		.optional()
		.into_diagnostic()?;

	let interaction_client = http_client.interaction(application_id);
	let Some(guild) = guild else {
		let response = InteractionResponseDataBuilder::new()
			.content("This server isn't set up for Twilight Sword.")
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
	};

	let create_ticket_instance_id = cuid2::create_id();
	let mut available_ticket_categories: Vec<(String, String)> = Vec::new();

	for built_in_category in BuiltInCategory::all_categories() {
		if !built_in_category.user_can_submit() || !built_in_category.is_enabled_for_guild(&guild) {
			continue;
		}
		let category_id = format!("default/{}", built_in_category.to_id());
		let category_name = built_in_category.name().to_string();
		available_ticket_categories.push((category_id, category_name));
	}

	let custom_categories: Vec<CustomCategory> = custom_categories::table
		.filter(custom_categories::guild.eq(db_guild_id))
		.load(&mut db_connection)
		.into_diagnostic()?;
	for category in custom_categories {
		available_ticket_categories.push((category.id, category.name));
	}

	let category_select_options: Vec<SelectMenuOption> = available_ticket_categories
		.into_iter()
		.map(|(id, name)| SelectMenuOption {
			default: false,
			description: None,
			emoji: None,
			label: name,
			value: id,
		})
		.collect();
	let category_select_menu = SelectMenu {
		channel_types: None,
		custom_id: format!("create_ticket/{}/set_category", create_ticket_instance_id),
		default_values: None,
		disabled: false,
		kind: SelectMenuType::Text,
		max_values: None,
		min_values: None,
		options: Some(category_select_options),
		placeholder: Some(String::from("Ticket category")),
	};
	let category_select = Component::SelectMenu(category_select_menu);
	let create_button = Button {
		custom_id: Some(format!("create_ticket/{}/confirm_category", create_ticket_instance_id)),
		disabled: true,
		emoji: None,
		label: Some(String::from("Create Ticket")),
		style: ButtonStyle::Primary,
		url: None,
		sku_id: None,
	};
	let create_button = Component::Button(create_button);

	let category_select_row = Component::ActionRow(ActionRow {
		components: vec![category_select],
	});
	let create_button_row = Component::ActionRow(ActionRow {
		components: vec![create_button],
	});

	{
		let mut state = bot_state.write().await;
		let create_ticket_states = state
			.entry::<CreateTicketStates>()
			.or_insert_with(CreateTicketStates::default);
		let create_ticket_state = CreateTicketState::new(&interaction.token);
		create_ticket_states
			.states
			.insert(create_ticket_instance_id.clone(), create_ticket_state);
	}
	tokio::spawn(expire_create(
		Arc::clone(http_client),
		application_id,
		bot_state,
		create_ticket_instance_id.clone(),
	));

	let response = InteractionResponseDataBuilder::new()
		.content("Select what type of ticket this is:")
		.components(vec![category_select_row, create_button_row])
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

	Ok(())
}

async fn expire_create(
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
	bot_state: Arc<RwLock<TypeMap>>,
	create_id: String,
) {
	sleep(MAX_INTERACTION_WAIT_TIME).await;
	let mut state = bot_state.write().await;
	let Some(create_ticket_states) = state.get_mut::<CreateTicketStates>() else {
		return;
	};
	let Some(create_ticket_state) = create_ticket_states.states.remove(&create_id) else {
		return;
	};

	let interaction_client = http_client.interaction(application_id);
	let _ = interaction_client
		.update_response(&create_ticket_state.initial_message_token)
		.content(Some("Ticket creation timed out."))
		.components(None)
		.await;
}
