// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::state::reply::ReplyStates;
use crate::discord::utils::timestamp::datetime_from_id;
use crate::model::{database_id_from_discord_id, TicketMessage};
use crate::schema::ticket_messages;
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{bail, IntoDiagnostic};
use std::future::IntoFuture;
use std::sync::Arc;
use tokio::sync::RwLock;
use twilight_http::client::Client;
use twilight_mention::fmt::Mention;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::channel::message::{AllowedMentions, MessageFlags};
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use twilight_util::builder::InteractionResponseDataBuilder;
use type_map::concurrent::TypeMap;

pub async fn route_reply_modal(
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
		bail!("Invalid custom ID for ticket creation (parts: {:?}", custom_id_path);
	};

	if action == "message" {
		handle_reply_modal(
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

async fn handle_reply_modal(
	interaction: &InteractionCreate,
	modal_data: &ModalInteractionData,
	reply_id: &str,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let mut message: Option<String> = None;

	for row in modal_data.components.iter() {
		for component in row.components.iter() {
			if component.custom_id.as_str() == "body" {
				message = component.value.clone()
			}
		}
	}

	let interaction_client = http_client.interaction(application_id);
	let Some(message) = message else {
		let response = InteractionResponseDataBuilder::new()
			.content("Reply not sent: missing required data.")
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

	let Some(message_author) = interaction.author_id() else {
		bail!("Modal submitted by a non-user");
	};

	let reply_state = {
		let mut states = bot_state.write().await;
		let reply_state = states
			.get_mut::<ReplyStates>()
			.and_then(|reply_states| reply_states.states.remove(reply_id));
		match reply_state {
			Some(state) => state,
			None => {
				let response = InteractionResponseDataBuilder::new()
					.content(format!(
						"Your reply to this ticket expired. In case you need it again, here's what you entered:\n{}",
						message
					))
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
		}
	};
	let ticket = reply_state.ticket;

	let response_content = format!("Ticket response from {}:\n\n{}", message_author.mention(), message);
	let response = InteractionResponseDataBuilder::new()
		.content(response_content)
		.allowed_mentions(AllowedMentions::default())
		.build();
	let response = InteractionResponse {
		kind: InteractionResponseType::ChannelMessageWithSource,
		data: Some(response),
	};
	let response_future = interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.into_future();

	let with_user = ticket.get_with_user();
	let user_message_content = format!("{}\n\n{}", with_user.mention(), message);
	let mut user_allowed_mentions = AllowedMentions::default();
	user_allowed_mentions.users.push(with_user);
	let user_thread = ticket.get_user_thread();
	let user_message_future = http_client
		.create_message(user_thread)
		.content(&user_message_content)
		.allowed_mentions(Some(&user_allowed_mentions))
		.into_future();

	let (response_result, user_message_result) = tokio::join!(response_future, user_message_future);
	response_result.into_diagnostic()?;
	let user_message_response = user_message_result.into_diagnostic()?;
	let user_message = user_message_response.model().await.into_diagnostic()?;
	let user_message = Some(database_id_from_discord_id(user_message.id.get()));

	let response_message = interaction_client
		.response(&interaction.token)
		.await
		.into_diagnostic()?;
	let response_message = response_message.model().await.into_diagnostic()?;

	let send_time = datetime_from_id(interaction.id).unwrap_or_else(Utc::now);
	let staff_message = database_id_from_discord_id(response_message.id.get());

	let ticket_message = TicketMessage {
		id: reply_id.to_string(),
		ticket: ticket.id,
		author: database_id_from_discord_id(message_author.get()),
		send_time,
		body: message.clone(),
		staff_message,
		user_message,
	};

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;

	diesel::insert_into(ticket_messages::table)
		.values(ticket_message)
		.execute(&mut db_connection)
		.into_diagnostic()?;

	Ok(())
}
