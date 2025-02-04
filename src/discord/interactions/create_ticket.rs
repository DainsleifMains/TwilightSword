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
use miette::{bail, ensure, IntoDiagnostic};
use std::future::IntoFuture;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::sleep;
use twilight_http::client::Client;
use twilight_mention::fmt::Mention;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::channel::message::component::{
	ActionRow, Button, ButtonStyle, Component, SelectMenu, SelectMenuOption, SelectMenuType, TextInput, TextInputStyle,
};
use twilight_model::channel::message::{AllowedMentions, MessageFlags};
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use twilight_util::builder::InteractionResponseDataBuilder;
use type_map::concurrent::TypeMap;

pub async fn route_create_ticket_interaction(
	interaction: &InteractionCreate,
	interaction_data: &MessageComponentInteractionData,
	custom_id_path: &[String],
	http_client: &Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let Some(id) = custom_id_path.get(1) else {
		bail!("Invalid custom ID for ticket creation (parts: {:?})", custom_id_path);
	};
	let Some(action) = custom_id_path.get(2) else {
		bail!("Invalid custom ID for ticket creation (parts: {:?}", custom_id_path);
	};

	match action.as_str() {
		"confirm_category" => {
			confirm_category(
				interaction,
				id,
				http_client,
				application_id,
				db_connection_pool,
				bot_state,
			)
			.await?
		}
		"set_category" => {
			set_category(
				interaction,
				interaction_data,
				id,
				http_client,
				application_id,
				db_connection_pool,
				bot_state,
			)
			.await?
		}
		"start" => {
			ensure!(id.is_empty(), "Unexpected ID when starting ticket creation");
			create_ticket(interaction, http_client, application_id, db_connection_pool, bot_state).await?;
		}
		_ => bail!(
			"Invalid action for ticket creation: {} (custom ID parts: {:?})",
			action,
			custom_id_path
		),
	}

	Ok(())
}

pub async fn route_create_ticket_modal(
	interaction: &InteractionCreate,
	modal_data: &ModalInteractionData,
	custom_id_path: &[String],
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let Some(id) = custom_id_path.get(1) else {
		bail!("Invalid custom ID for ticket creation (parts: {:?})", custom_id_path);
	};
	let Some(action) = custom_id_path.get(2) else {
		bail!("Invalid custom ID for ticket creation (parts: {:?})", custom_id_path);
	};

	if action == "message" {
		handle_message_modal_data(
			interaction,
			modal_data,
			id,
			http_client,
			application_id,
			db_connection_pool,
			bot_state,
		)
		.await?;
	} else {
		bail!(
			"Invalid action for ticket creation: {} (custom ID parts: {:?})",
			action,
			custom_id_path
		);
	}

	Ok(())
}

async fn create_ticket(
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

	let available_ticket_categories = selectable_categories_for_guild(&guild, &mut db_connection)?;
	if available_ticket_categories.is_empty() {
		let response = InteractionResponseDataBuilder::new()
			.content("Tickets can't be created on this server at this time.")
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
	let category_components =
		category_select_components(&create_ticket_instance_id, available_ticket_categories, true, None);

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
		.components(category_components)
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

fn selectable_categories_for_guild(
	guild: &Guild,
	db_connection: &mut PgConnection,
) -> miette::Result<Vec<(String, String)>> {
	let mut available_ticket_categories: Vec<(String, String)> = Vec::new();

	for built_in_category in BuiltInCategory::all_categories() {
		if !built_in_category.user_can_submit_from_server() || !built_in_category.is_enabled_for_guild(guild) {
			continue;
		}
		let category_id = format!("default/{}", built_in_category.as_id());
		let category_name = built_in_category.name().to_string();
		available_ticket_categories.push((category_id, category_name));
	}

	let custom_categories: Vec<CustomCategory> = custom_categories::table
		.filter(custom_categories::guild.eq(guild.guild_id))
		.load(db_connection)
		.into_diagnostic()?;
	for category in custom_categories {
		available_ticket_categories.push((category.id, category.name));
	}

	Ok(available_ticket_categories)
}

fn category_select_components(
	create_id: &str,
	available_ticket_categories: Vec<(String, String)>,
	create_button_disabled: bool,
	selected_category_id: Option<&str>,
) -> Vec<Component> {
	let category_select_options: Vec<SelectMenuOption> = available_ticket_categories
		.into_iter()
		.map(|(id, name)| SelectMenuOption {
			default: Some(id.as_str()) == selected_category_id,
			description: None,
			emoji: None,
			label: name,
			value: id,
		})
		.collect();
	let category_select_menu = SelectMenu {
		channel_types: None,
		custom_id: format!("create_ticket/{}/set_category", create_id),
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
		custom_id: Some(format!("create_ticket/{}/confirm_category", create_id)),
		disabled: create_button_disabled,
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

	vec![category_select_row, create_button_row]
}

async fn set_category(
	interaction: &InteractionCreate,
	interaction_data: &MessageComponentInteractionData,
	create_id: &str,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let mut state = bot_state.write().await;
	let Some(create_ticket_states) = state.get_mut::<CreateTicketStates>() else {
		bail!("Failed to get ticket creation states responding to interaction");
	};
	let Some(create_ticket_state) = create_ticket_states.states.get_mut(create_id) else {
		bail!(
			"Failed to get ticket creation state for ID {} responding to interaction",
			create_id
		);
	};

	let Some(category_id) = interaction_data.values.first() else {
		bail!("Missing category selection handling ticket creation event");
	};

	if let Some(built_in_category_id) = category_id.strip_prefix("default/") {
		let Some(built_in_category) = BuiltInCategory::from_id(built_in_category_id) else {
			bail!("Invalid built-in category passed to ticket creation");
		};
		create_ticket_state.built_in_category = Some(built_in_category);
		create_ticket_state.custom_category_id = None;
	} else {
		create_ticket_state.built_in_category = None;
		create_ticket_state.custom_category_id = Some(category_id.clone());
	}

	let interaction_client = http_client.interaction(application_id);
	let acknowledge_response = InteractionResponse {
		kind: InteractionResponseType::DeferredUpdateMessage,
		data: None,
	};
	let acknowledge_future = interaction_client
		.create_response(interaction.id, &interaction.token, &acknowledge_response)
		.into_future();

	let Some(guild_id) = interaction.guild_id else {
		bail!("Ticket creation interaction moved outside guild");
	};
	let db_guild_id = database_id_from_discord_id(guild_id.get());
	let mut db_connection = db_connection_pool.get().into_diagnostic()?;
	let guild: Option<Guild> = guilds::table
		.find(db_guild_id)
		.first(&mut db_connection)
		.optional()
		.into_diagnostic()?;
	let Some(guild) = guild else {
		bail!("In ticket creation flow, guild is no longer set up");
	};

	let selectable_categories = selectable_categories_for_guild(&guild, &mut db_connection)?;
	if selectable_categories.is_empty() {
		interaction_client
			.update_response(&create_ticket_state.initial_message_token)
			.content(Some("This server is no longer accepting new tickets."))
			.components(None)
			.await
			.into_diagnostic()?;
		return Ok(());
	}

	let updated_components = category_select_components(create_id, selectable_categories, false, Some(category_id));
	let update_future = interaction_client
		.update_response(&create_ticket_state.initial_message_token)
		.components(Some(&updated_components))
		.into_future();

	drop(state);

	let (acknowledge_result, update_result) = tokio::join!(acknowledge_future, update_future);
	acknowledge_result.into_diagnostic()?;
	update_result.into_diagnostic()?;

	Ok(())
}

async fn confirm_category(
	interaction: &InteractionCreate,
	create_id: &str,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let (selected_built_in_category, selected_custom_category) = {
		let state = bot_state.read().await;
		let Some(create_ticket_states) = state.get::<CreateTicketStates>() else {
			bail!("Confirming category when no ticket creation states have been created.");
		};
		let Some(create_ticket_state) = create_ticket_states.states.get(create_id) else {
			// In this situation, the most likely situation is that the original token expired as this message came in.
			// This is benign and the expiration process should notify the user.
			return Ok(());
		};
		(
			create_ticket_state.built_in_category,
			create_ticket_state.custom_category_id.clone(),
		)
	};

	let category_name = match (selected_built_in_category, selected_custom_category) {
		(Some(category), _) => category.name().to_string(),
		(_, Some(category_id)) => {
			let mut db_connection = db_connection_pool.get().into_diagnostic()?;
			let category: CustomCategory = custom_categories::table
				.find(&category_id)
				.first(&mut db_connection)
				.into_diagnostic()?;
			category.name
		}
		_ => String::new(),
	};

	let interaction_client = http_client.interaction(application_id);

	let mut components: Vec<Component> = Vec::new();
	if let Some(BuiltInCategory::NewPartner) = selected_built_in_category {
		let invite_input = Component::TextInput(TextInput {
			custom_id: String::from("invite_url"),
			label: String::from("Server invite URL"),
			max_length: None,
			min_length: Some(10),
			placeholder: Some(String::from("Invite URL")),
			required: Some(true),
			style: TextInputStyle::Short,
			value: None,
		});
		let invite_row = Component::ActionRow(ActionRow {
			components: vec![invite_input],
		});
		components.push(invite_row);
	}

	let title_input = Component::TextInput(TextInput {
		custom_id: String::from("title"),
		label: String::from("Ticket Title"),
		max_length: None,
		min_length: None,
		placeholder: Some(String::from("Title")),
		required: Some(true),
		style: TextInputStyle::Short,
		value: None,
	});
	let title_row = Component::ActionRow(ActionRow {
		components: vec![title_input],
	});
	components.push(title_row);

	let body_input = Component::TextInput(TextInput {
		custom_id: String::from("body"),
		label: String::from("Message"),
		max_length: None,
		min_length: None,
		placeholder: None,
		required: Some(true),
		style: TextInputStyle::Paragraph,
		value: None,
	});
	let body_row = Component::ActionRow(ActionRow {
		components: vec![body_input],
	});
	components.push(body_row);

	let modal_id = format!("create_ticket/{}/message", create_id);
	let response = InteractionResponseDataBuilder::new()
		.custom_id(modal_id)
		.title(format!("Create Ticket - {}", category_name))
		.components(components)
		.build();
	let response = InteractionResponse {
		kind: InteractionResponseType::Modal,
		data: Some(response),
	};
	interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.await
		.into_diagnostic()?;

	Ok(())
}

async fn handle_message_modal_data(
	interaction: &InteractionCreate,
	modal_data: &ModalInteractionData,
	create_id: &str,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let mut invite_url: Option<String> = None;
	let mut ticket_title: Option<String> = None;
	let mut ticket_message: Option<String> = None;

	for row in modal_data.components.iter() {
		for component in row.components.iter() {
			match component.custom_id.as_str() {
				"invite_url" => invite_url = component.value.clone(),
				"title" => ticket_title = component.value.clone(),
				"body" => ticket_message = component.value.clone(),
				_ => (),
			}
		}
	}

	let interaction_client = http_client.interaction(application_id);
	let (Some(ticket_title), Some(ticket_message)) = (ticket_title, ticket_message) else {
		let response = InteractionResponseDataBuilder::new()
			.content("Ticket not sent: missing required data.")
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

	let create_ticket_state = {
		let mut state = bot_state.write().await;
		let Some(create_ticket_states) = state.get_mut::<CreateTicketStates>() else {
			bail!("Confirming ticket creation with no ticket creation state data");
		};
		let Some(create_ticket_state) = create_ticket_states.states.remove(create_id) else {
			let message_content = format!("Failed to get ticket submission state; your ticket entry may have expired. Please try again.\nIf you need it, here's the data you entered:\n\n**Title**: {}\n**Message**:\n{}", ticket_title, ticket_message);
			let response = InteractionResponseDataBuilder::new()
				.content(message_content)
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
		create_ticket_state
	};

	let ticket_message = if let Some(BuiltInCategory::NewPartner) = create_ticket_state.built_in_category {
		let Some(invite_url) = &invite_url else {
			bail!("Invite URL not entered on new partner ticket");
		};
		format!("**Partner invite URL**: {}\n\n{}", invite_url, ticket_message)
	} else {
		ticket_message
	};

	let Some(guild_id) = interaction.guild_id else {
		bail!("Create ticket workflow moved outside of a guild");
	};
	let db_guild_id = database_id_from_discord_id(guild_id.get());
	let Some(interaction_member) = &interaction.member else {
		bail!("Interaction isn't from a user");
	};
	let Some(interaction_user) = &interaction_member.user else {
		bail!("Guild member doesn't have a user");
	};
	let db_user_id = database_id_from_discord_id(interaction_user.id.get());
	let new_ticket = Ticket {
		id: create_id.to_string(),
		guild: db_guild_id,
		with_user: db_user_id,
		title: ticket_title.clone(),
		built_in_category: create_ticket_state
			.built_in_category
			.map(|category| category.to_database()),
		custom_category: create_ticket_state.custom_category_id.clone(),
		is_open: true,
	};

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;
	let guild_data: Option<Guild> = guilds::table
		.find(db_guild_id)
		.first(&mut db_connection)
		.optional()
		.into_diagnostic()?;
	let Some(guild_data) = guild_data else {
		let response = InteractionResponseDataBuilder::new()
			.content("This server is not set up and cannot accept tickets.")
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

	let ticket_thread_message_content = if let Some(invite_url) = &invite_url {
		format!(
			"**Author**: {}\n**Invite URL**: {}\n\n{}",
			interaction_user.mention(),
			invite_url,
			ticket_message
		)
	} else {
		format!("**Author**: {}\n\n{}", interaction_user.mention(), ticket_message)
	};

	let channel_id = match (
		create_ticket_state.built_in_category,
		create_ticket_state.custom_category_id,
	) {
		(Some(BuiltInCategory::NewPartner), _) => guild_data.get_new_partner_ticket_channel(),
		(Some(BuiltInCategory::ExistingPartner), _) => guild_data.get_existing_partner_ticket_channel(),
		(_, Some(custom_category)) => {
			let custom_category: CustomCategory = custom_categories::table
				.find(&custom_category)
				.first(&mut db_connection)
				.into_diagnostic()?;
			Some(custom_category.get_channel())
		}
		_ => bail!("Invalid category selection for new ticket creation"),
	};

	let Some(channel_id) = channel_id else {
		let response = InteractionResponseDataBuilder::new()
			.content("The server is no longer accepting tickets of that category.")
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

	let ticket_thread_result = http_client
		.create_forum_thread(channel_id, &ticket_title)
		.message()
		.content(&ticket_thread_message_content)
		.allowed_mentions(Some(&AllowedMentions::default()))
		.await;
	if ticket_thread_result.is_err() {
		let response_content = format!("This ticket couldn't be sent. In case you want it later, here's what you sent:\n**Title**: {}\n**Message**\n{}", ticket_title, ticket_message);
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
		return Ok(());
	}

	diesel::insert_into(tickets::table)
		.values(new_ticket)
		.execute(&mut db_connection)
		.into_diagnostic()?;

	let response = InteractionResponseDataBuilder::new()
		.content("Ticket submitted!")
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
