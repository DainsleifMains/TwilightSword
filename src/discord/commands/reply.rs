// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::state::reply::{ReplyState, ReplyStates};
use crate::model::{database_id_from_discord_id, Ticket};
use crate::schema::tickets;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::IntoDiagnostic;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use twilight_http::client::Client;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::channel::message::component::{ActionRow, Component, TextInput, TextInputStyle};
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use twilight_util::builder::command::CommandBuilder;
use twilight_util::builder::InteractionResponseDataBuilder;
use type_map::concurrent::TypeMap;

pub fn command_definition() -> Command {
	CommandBuilder::new("reply", "Reply to a ticket", CommandType::ChatInput)
		.contexts([InteractionContextType::Guild])
		.build()
}

pub async fn handle_command(
	interaction: &InteractionCreate,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let mut db_connection = db_connection_pool.get().into_diagnostic()?;
	let ticket = match interaction.channel.as_ref() {
		Some(channel) => {
			let channel_id = channel.id;
			let db_channel_id = database_id_from_discord_id(channel_id.get());
			let ticket: Option<Ticket> = tickets::table
				.filter(tickets::staff_thread.eq(db_channel_id))
				.first(&mut db_connection)
				.optional()
				.into_diagnostic()?;
			ticket
		}
		None => None,
	};

	let interaction_client = http_client.interaction(application_id);
	let Some(ticket) = ticket else {
		let response = InteractionResponseDataBuilder::new()
			.content("This command is only useful in a staff ticket thread.")
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

	let reply_id = cuid2::create_id();
	let new_state = ReplyState { ticket };
	let mut states = bot_state.write().await;
	let reply_states = states.entry().or_insert_with(ReplyStates::default);
	reply_states.states.insert(reply_id.clone(), new_state);

	let modal_id = format!("reply/{}/message", reply_id);

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
	let body_input_row = Component::ActionRow(ActionRow {
		components: vec![body_input],
	});
	let response = InteractionResponseDataBuilder::new()
		.custom_id(modal_id)
		.title("Ticket Reply")
		.components(vec![body_input_row])
		.build();
	let response = InteractionResponse {
		kind: InteractionResponseType::Modal,
		data: Some(response),
	};
	interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.await
		.into_diagnostic()?;

	drop(states);

	tokio::spawn(expire_reply(bot_state, reply_id));

	Ok(())
}

async fn expire_reply(bot_state: Arc<RwLock<TypeMap>>, reply_id: String) {
	sleep(Duration::from_secs(3600)).await;
	let mut states = bot_state.write().await;
	let Some(reply_states) = states.get_mut::<ReplyStates>() else {
		return;
	};
	reply_states.states.remove(&reply_id);
}
