// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::state::settings::start_ticket_message::StartTicketMessageState;
use crate::discord::utils::shared_components::new_ticket_button;
use crate::model::{database_id_from_discord_id, Guild};
use crate::schema::guilds;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{bail, IntoDiagnostic};
use std::sync::Arc;
use tokio::sync::RwLock;
use twilight_http::client::Client;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::EmbedBuilder;
use twilight_util::builder::InteractionResponseDataBuilder;
use type_map::concurrent::TypeMap;

pub async fn handle_start_ticket_message_modal(
	interaction: &InteractionCreate,
	modal_data: &ModalInteractionData,
	custom_id_path: &[String],
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let Some(modal_id) = custom_id_path.get(2) else {
		bail!("Received modal with no ID");
	};

	let interaction_client = http_client.interaction(application_id);
	let guild_id = {
		let mut state = bot_state.write().await;
		let Some(start_message_guilds) = state.get_mut::<StartTicketMessageState>() else {
			bail!("Modal response invoked with no modal data");
		};
		let Some(guild_id) = start_message_guilds.guilds.remove(modal_id) else {
			let response = InteractionResponseDataBuilder::new()
				.content("The modal interaction has expired. Please try again.")
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
		guild_id
	};

	let mut new_message = String::new();

	for action_row in modal_data.components.iter() {
		for component in action_row.components.iter() {
			if component.custom_id == "message" {
				new_message = component.value.clone().unwrap_or_default();
			}
		}
	}

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;
	let db_guild_id = database_id_from_discord_id(guild_id.get());
	let db_result: QueryResult<Guild> = diesel::update(guilds::table)
		.filter(guilds::guild_id.eq(db_guild_id))
		.set(guilds::start_ticket_message.eq(&new_message))
		.get_result(&mut db_connection);
	let guild = match db_result {
		Ok(guild) => {
			let embed = EmbedBuilder::new().title("Message").description(&new_message).build();
			let response = InteractionResponseDataBuilder::new()
				.content("Start ticket message updated.")
				.embeds([embed])
				.build();
			let response = InteractionResponse {
				kind: InteractionResponseType::ChannelMessageWithSource,
				data: Some(response),
			};
			interaction_client
				.create_response(interaction.id, &interaction.token, &response)
				.await
				.into_diagnostic()?;
			guild
		}
		Err(error) => {
			tracing::error!(source = ?error, "Failed to update start ticket message");
			let response = InteractionResponseDataBuilder::new()
				.content("An internal error prevented updating the message.")
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

	if let Some(start_ticket_channel_id) = guild.get_start_ticket_channel() {
		match guild.get_start_ticket_message_id() {
			Some(message_id) => {
				http_client
					.update_message(start_ticket_channel_id, message_id)
					.content(Some(&new_message))
					.await
					.into_diagnostic()?;
			}
			None => {
				let new_ticket_button = new_ticket_button(guild_id);
				let new_message = http_client
					.create_message(start_ticket_channel_id)
					.content(&new_message)
					.components(&[new_ticket_button])
					.await
					.into_diagnostic()?
					.model()
					.await
					.into_diagnostic()?;
				let new_message_id = new_message.id;
				let db_message_id = database_id_from_discord_id(new_message_id.get());
				diesel::update(guilds::table)
					.filter(guilds::guild_id.eq(db_guild_id))
					.set(guilds::start_ticket_message_id.eq(db_message_id))
					.execute(&mut db_connection)
					.into_diagnostic()?;
			}
		}
	}

	Ok(())
}
