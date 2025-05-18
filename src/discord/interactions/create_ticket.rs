// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::state::create_ticket::{BuiltInCategory, CreateTicketState, CreateTicketStates};
use crate::discord::utils::invites::invite_code_from_url;
use crate::discord::utils::tickets::{MAX_TICKET_TITLE_LENGTH, UserMessageAuthor, staff_message, user_message};
use crate::discord::utils::timestamp::timestamp_from_id;
use crate::model::{
	CustomCategory, FormQuestion, Guild, PendingPartnership, Ticket, TicketMessage, TicketRestrictedUser,
	database_id_from_discord_id,
};
use crate::schema::{
	custom_categories, form_questions, guilds, pending_partnerships, ticket_messages, ticket_restricted_users, tickets,
};
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error as DbError;
use miette::{IntoDiagnostic, bail, ensure};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};
use twilight_http::client::Client;
use twilight_http::error::ErrorType;
use twilight_http::response::StatusCode;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::channel::ChannelType;
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::message::component::{
	ActionRow, Button, ButtonStyle, Component, SelectMenu, SelectMenuOption, SelectMenuType, TextInput, TextInputStyle,
};
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use twilight_util::builder::InteractionResponseDataBuilder;
use type_map::concurrent::TypeMap;

const TICKET_CREATION_EXPIRED: &str = "Ticket creation expired.";

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

	let interaction_client = http_client.interaction(application_id);
	let Some(interaction_member) = &interaction.member else {
		bail!("Interaction isn't from a user");
	};
	let Some(interaction_user) = &interaction_member.user else {
		bail!("Interaction member is not a user");
	};
	let db_user_id = database_id_from_discord_id(interaction_user.id.get());

	let restriction: Option<TicketRestrictedUser> = ticket_restricted_users::table
		.find((db_guild_id, db_user_id))
		.first(&mut db_connection)
		.optional()
		.into_diagnostic()?;
	if restriction.is_some() {
		let response = InteractionResponseDataBuilder::new()
			.content("You may not send tickets on this server.")
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

	let guild: Option<Guild> = guilds::table
		.find(db_guild_id)
		.first(&mut db_connection)
		.optional()
		.into_diagnostic()?;

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
		let create_ticket_state = CreateTicketState::default();
		create_ticket_states
			.states
			.insert(create_ticket_instance_id.clone(), create_ticket_state);
	}
	tokio::spawn(expire_create(bot_state, create_ticket_instance_id.clone()));

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

async fn expire_create(bot_state: Arc<RwLock<TypeMap>>, create_id: String) {
	sleep(Duration::from_secs(3600)).await;
	let mut state = bot_state.write().await;
	if let Some(create_ticket_states) = state.get_mut::<CreateTicketStates>() {
		create_ticket_states.states.remove(&create_id);
	};
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
		let category_name = format!("{}", built_in_category);
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
	let interaction_client = http_client.interaction(application_id);
	let Some(create_ticket_state) = create_ticket_states.states.get_mut(create_id) else {
		let response = InteractionResponseDataBuilder::new()
			.content(TICKET_CREATION_EXPIRED)
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

	drop(state);

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
		let response = InteractionResponseDataBuilder::new()
			.content("This server is no longer accepting new tickets.")
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

	let updated_components = category_select_components(create_id, selectable_categories, false, Some(category_id));
	let response = InteractionResponseDataBuilder::new()
		.components(updated_components)
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

async fn confirm_category(
	interaction: &InteractionCreate,
	create_id: &str,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let interaction_client = http_client.interaction(application_id);

	let (selected_built_in_category, selected_custom_category) = {
		let state = bot_state.read().await;
		let Some(create_ticket_states) = state.get::<CreateTicketStates>() else {
			bail!("Confirming category when no ticket creation states have been created.");
		};
		let Some(create_ticket_state) = create_ticket_states.states.get(create_id) else {
			let response = InteractionResponseDataBuilder::new()
				.content(TICKET_CREATION_EXPIRED)
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
		(
			create_ticket_state.built_in_category,
			create_ticket_state.custom_category_id.clone(),
		)
	};

	let Some(guild_id) = interaction.guild_id else {
		bail!("Ticket creation moved outside of a guild");
	};
	let db_guild_id = database_id_from_discord_id(guild_id.get());
	let mut db_connection = db_connection_pool.get().into_diagnostic()?;

	let (category_name, category_form_id) = match (selected_built_in_category, selected_custom_category) {
		(Some(category), _) => {
			let guild_data: Guild = guilds::table
				.find(db_guild_id)
				.first(&mut db_connection)
				.into_diagnostic()?;
			let form_id = match category {
				BuiltInCategory::BanAppeal => guild_data.ban_appeal_ticket_form,
				BuiltInCategory::NewPartner => guild_data.new_partner_ticket_form,
				BuiltInCategory::ExistingPartner => guild_data.existing_partner_ticket_form,
				BuiltInCategory::MessageReport => None,
			};
			(format!("{}", category), form_id)
		}
		(_, Some(category_id)) => {
			let category: CustomCategory = custom_categories::table
				.find(&category_id)
				.first(&mut db_connection)
				.into_diagnostic()?;
			(category.name, category.form)
		}
		_ => (String::new(), None),
	};

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
		max_length: Some(MAX_TICKET_TITLE_LENGTH),
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

	match category_form_id {
		Some(form_id) => {
			let form_questions: Vec<FormQuestion> = form_questions::table
				.filter(form_questions::form.eq(&form_id))
				.order(form_questions::form_position.asc())
				.load(&mut db_connection)
				.into_diagnostic()?;
			let max_separate_questions = if let Some(BuiltInCategory::NewPartner) = selected_built_in_category {
				// This category has an extra field for the invite URL, which restricts the number of questions we can show as separate fields in the embed.
				3
			} else {
				4
			};
			if form_questions.len() <= max_separate_questions {
				for question in form_questions {
					let question_input = Component::TextInput(TextInput {
						custom_id: format!("question/{}", question.id),
						label: question.question.clone(),
						max_length: None,
						min_length: None,
						placeholder: None,
						required: Some(true),
						style: TextInputStyle::Paragraph,
						value: None,
					});
					let question_row = Component::ActionRow(ActionRow {
						components: vec![question_input],
					});
					components.push(question_row);
				}
			} else {
				let mut body = String::new();
				for question in form_questions {
					let question = question.question.replace("*", "\\*");
					body = format!("{}**{}**\n\n\n", body, question);
				}
				let body_input = Component::TextInput(TextInput {
					custom_id: String::from("body"),
					label: String::from("Message"),
					max_length: None,
					min_length: None,
					placeholder: None,
					required: Some(true),
					style: TextInputStyle::Paragraph,
					value: Some(body),
				});
				let body_row = Component::ActionRow(ActionRow {
					components: vec![body_input],
				});
				components.push(body_row);
			}
		}
		None => {
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
		}
	}

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

fn try_again_text(ticket_title: &str, ticket_message: &str) -> String {
	format!(
		"If you wish to try again, here's the data you submitted:\n\n**Title**: {}\n**Message**:\n{}",
		ticket_title, ticket_message
	)
}

struct QuestionAnswer {
	question: String,
	position: i32,
	answer: String,
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
	let mut question_answer_data: Vec<QuestionAnswer> = Vec::new();

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;

	for row in modal_data.components.iter() {
		for component in row.components.iter() {
			if let Some(question_id) = component.custom_id.strip_prefix("question/") {
				let question_data: FormQuestion = form_questions::table
					.find(question_id)
					.first(&mut db_connection)
					.into_diagnostic()?;
				let answer_data = QuestionAnswer {
					question: question_data.question.clone(),
					position: question_data.form_position,
					answer: component.value.clone().unwrap_or_default(),
				};
				question_answer_data.push(answer_data);
				continue;
			}
			match component.custom_id.as_str() {
				"invite_url" => invite_url = component.value.clone(),
				"title" => ticket_title = component.value.clone(),
				"body" => ticket_message = component.value.clone(),
				_ => (),
			}
		}
	}

	if ticket_message.is_none() && !question_answer_data.is_empty() {
		let mut new_message = String::new();
		question_answer_data.sort_by_key(|data| data.position);
		for data in question_answer_data {
			let question = data.question.replace("*", "\\*");
			if new_message.is_empty() {
				new_message = format!("**{}**\n{}", question, data.answer);
			} else {
				new_message = format!("{}\n\n**{}**\n{}", new_message, question, data.answer);
			}
		}
		ticket_message = Some(new_message);
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

	let max_ticket_title_len: usize = MAX_TICKET_TITLE_LENGTH.into();
	if ticket_title.len() > max_ticket_title_len {
		let response = format!(
			"Your ticket couldn't be sent, as the title is too long.\n{}",
			try_again_text(&ticket_title, &ticket_message)
		);
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
		return Ok(());
	}

	let create_ticket_state = {
		let mut state = bot_state.write().await;
		let Some(create_ticket_states) = state.get_mut::<CreateTicketStates>() else {
			bail!("Confirming ticket creation with no ticket creation state data");
		};
		let Some(create_ticket_state) = create_ticket_states.states.remove(create_id) else {
			let message_content = format!(
				"Ticket creation expired.\n{}",
				try_again_text(&ticket_title, &ticket_message)
			);
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

	let (ticket_message, invite_data) = match create_ticket_state.built_in_category {
		Some(BuiltInCategory::NewPartner) => {
			let Some(invite_url) = &invite_url else {
				bail!("Invite URL not entered on new partner ticket");
			};

			let Some(invite_code) = invite_code_from_url(invite_url) else {
				let response_message = format!(
					"The invite URL you provided is not a valid invite.\n{}",
					try_again_text(&ticket_title, &ticket_message)
				);
				let response = InteractionResponseDataBuilder::new()
					.content(response_message)
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

			let invite_data = http_client.invite(&invite_code).with_expiration().await;
			if let Err(invite_error) = &invite_data {
				if let ErrorType::Response {
					status: StatusCode::NOT_FOUND,
					..
				} = invite_error.kind()
				{
					let response_message = format!(
						"Discord doesn't recognize that invite code.\n{}",
						try_again_text(&ticket_title, &ticket_message)
					);
					let response = InteractionResponseDataBuilder::new()
						.content(response_message)
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
			}
			let invite_data = invite_data.into_diagnostic()?;
			let invite_data = invite_data.model().await.into_diagnostic()?;

			if invite_data.guild.is_none() {
				let response_message = format!(
					"The invite you provided isn't for a guild.\n{}",
					try_again_text(&ticket_title, &ticket_message)
				);
				let response = InteractionResponseDataBuilder::new()
					.content(response_message)
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
			if invite_data.expires_at.is_some() {
				let response_message = format!(
					"The invite you provided expires, but partnership invites must be permanent.\n{}",
					try_again_text(&ticket_title, &ticket_message)
				);
				let response = InteractionResponseDataBuilder::new()
					.content(response_message)
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

			(
				format!("**Partner invite URL**: {}\n\n{}", invite_url, ticket_message),
				Some(invite_data),
			)
		}
		_ => (ticket_message, None),
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
	let Some(create_ticket_channel) = guild_data.get_start_ticket_channel() else {
		let response = InteractionResponseDataBuilder::new()
			.content("This server's ticket system is disabled.")
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

	let message_sent_timestamp = timestamp_from_id(interaction.id).into_diagnostic()?;

	let staff_ticket_message_data = match staff_message(&interaction_user.name, &ticket_message, message_sent_timestamp)
	{
		Ok(data) => data,
		Err(_) => {
			let response = InteractionResponseDataBuilder::new()
				.content("Your ticket couldn't be sent; its contents don't fit in an embed.")
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
	};

	let staff_channel_id = match (
		create_ticket_state.built_in_category,
		create_ticket_state.custom_category_id.clone(),
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

	let Some(staff_channel_id) = staff_channel_id else {
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

	let user_ticket_thread_response = http_client
		.create_thread(create_ticket_channel, &ticket_title, ChannelType::PrivateThread)
		.invitable(false)
		.await
		.into_diagnostic()?;
	let user_ticket_thread = user_ticket_thread_response.model().await.into_diagnostic()?;
	http_client
		.add_thread_member(user_ticket_thread.id, interaction_user.id)
		.await
		.into_diagnostic()?;

	let staff_ticket_title = format!("{} [{}]", ticket_title, interaction_user.name);
	let mut staff_ticket_message = http_client
		.create_forum_thread(staff_channel_id, &staff_ticket_title)
		.message();
	if let Some(content) = &staff_ticket_message_data.content {
		staff_ticket_message = staff_ticket_message.content(content);
	}
	staff_ticket_message = staff_ticket_message
		.embeds(&staff_ticket_message_data.embeds)
		.allowed_mentions(Some(&staff_ticket_message_data.allowed_mentions));
	let staff_ticket_thread_future = staff_ticket_message.into_future();

	let user_ticket_author = UserMessageAuthor::User(interaction_user.name.clone());
	let user_ticket_message_data = user_message(
		user_ticket_author,
		interaction_user.id,
		false,
		&ticket_message,
		message_sent_timestamp,
	)
	.into_diagnostic()?;
	let mut user_ticket_create_message = http_client.create_message(user_ticket_thread.id);
	user_ticket_create_message = user_ticket_message_data.set_create_message_data(user_ticket_create_message);
	let user_ticket_message_future = user_ticket_create_message.into_future();

	let (staff_ticket_thread_result, user_ticket_message_result) =
		tokio::join!(staff_ticket_thread_future, user_ticket_message_future);

	let staff_ticket_thread_response = match staff_ticket_thread_result {
		Ok(response) => response,
		Err(_) => {
			let response_content = format!(
				"This ticket couldn't be sent. In case you want it later, here's what you sent:\n**Title**: {}\n**Message**\n{}",
				ticket_title, ticket_message
			);
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
	};
	let user_ticket_message_response = match user_ticket_message_result {
		Ok(response) => response,
		Err(_) => {
			let response_content = format!(
				"This ticket couldn't be sent. In case you want it later, here's what you sent:\n**Title**: {}\n**Message**:\n{}",
				ticket_title, ticket_message
			);
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
	};

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

	let staff_ticket_thread = staff_ticket_thread_response.model().await.into_diagnostic()?;
	let db_staff_thread_id = database_id_from_discord_id(staff_ticket_thread.channel.id.get());
	let db_staff_message_id = database_id_from_discord_id(staff_ticket_thread.message.id.get());

	let user_ticket_message = user_ticket_message_response.model().await.into_diagnostic()?;
	let db_user_thread_id = database_id_from_discord_id(user_ticket_thread.id.get());
	let db_user_message_id = database_id_from_discord_id(user_ticket_message.id.get());

	let db_user_id = database_id_from_discord_id(interaction_user.id.get());
	let new_ticket = Ticket {
		id: create_id.to_string(),
		guild: db_guild_id,
		with_user: db_user_id,
		title: ticket_title.clone(),
		built_in_category: create_ticket_state
			.built_in_category
			.map(|category| category.to_database()),
		custom_category: create_ticket_state.custom_category_id,
		staff_thread: db_staff_thread_id,
		user_thread: db_user_thread_id,
		closed_at: None,
	};
	let new_ticket_message = TicketMessage {
		id: cuid2::create_id(),
		ticket: create_id.to_string(),
		author: db_user_id,
		send_time: Utc::now(),
		body: ticket_message.clone(),
		staff_message: db_staff_message_id,
		user_message: Some(db_user_message_id),
	};
	let pending_partnership = invite_data.map(|invite_data| PendingPartnership {
		id: cuid2::create_id(),
		guild: db_guild_id,
		partner_guild: database_id_from_discord_id(invite_data.guild.unwrap().id.get()),
		invite_code: invite_data.code,
		ticket: create_id.to_string(),
	});

	db_connection
		.transaction(|db_connection| {
			diesel::insert_into(tickets::table)
				.values(new_ticket)
				.execute(db_connection)?;
			diesel::insert_into(ticket_messages::table)
				.values(new_ticket_message)
				.execute(db_connection)?;
			if let Some(pending_partnership) = pending_partnership {
				diesel::insert_into(pending_partnerships::table)
					.values(pending_partnership)
					.execute(db_connection)?;
			}
			Ok::<(), DbError>(())
		})
		.into_diagnostic()?;

	Ok(())
}
