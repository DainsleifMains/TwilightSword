// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::state::settings::start_ticket_message::StartTicketMessageState;
use crate::discord::utils::responses::NOT_SET_UP_FOR_GUILD;
use crate::model::{database_id_from_discord_id, Guild};
use crate::schema::guilds;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{bail, IntoDiagnostic};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use twilight_http::client::Client;
use twilight_model::application::command::CommandOption;
use twilight_model::channel::message::component::{ActionRow, Component, TextInput, TextInputStyle};
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use twilight_util::builder::command::SubCommandBuilder;
use twilight_util::builder::InteractionResponseDataBuilder;
use type_map::concurrent::TypeMap;

pub fn subcommand_definition() -> CommandOption {
	SubCommandBuilder::new(
		"start_ticket_message",
		"The message used along with the \"create ticket\" button in the start ticket channel.",
	)
	.build()
}

pub async fn handle_subcommand(
	interaction: &InteractionCreate,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let Some(guild_id) = interaction.guild_id else {
		bail!("Settings command was used outside of a guild");
	};

	let db_guild_id = database_id_from_discord_id(guild_id.get());
	let mut db_connection = db_connection_pool.get().into_diagnostic()?;
	let guild: QueryResult<Option<Guild>> = guilds::table.find(db_guild_id).first(&mut db_connection).optional();

	let interaction_client = http_client.interaction(application_id);

	let guild = match guild {
		Ok(Some(guild)) => guild,
		Ok(None) => {
			let response = InteractionResponseDataBuilder::new()
				.content(NOT_SET_UP_FOR_GUILD)
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
		Err(error) => {
			tracing::error!(source = ?error, "Failed to get guild data for `/settings start_ticket_message`");
			let response = InteractionResponseDataBuilder::new()
				.content("An internal error prevented reading the current setting value.")
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

	let prompt_id = cuid2::create_id();
	let modal_id = format!("settings/start_ticket_message/{}/modal", prompt_id);

	let text_input = Component::TextInput(TextInput {
		custom_id: String::from("message"),
		label: String::from("Message contents"),
		max_length: None,
		min_length: None,
		placeholder: None,
		required: Some(false),
		style: TextInputStyle::Paragraph,
		value: Some(guild.start_ticket_message.clone()),
	});
	let text_input_row = Component::ActionRow(ActionRow {
		components: vec![text_input],
	});
	let response = InteractionResponseDataBuilder::new()
		.custom_id(modal_id)
		.title("Start Ticket Message")
		.components(vec![text_input_row])
		.build();
	let response = InteractionResponse {
		kind: InteractionResponseType::Modal,
		data: Some(response),
	};
	interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.await
		.into_diagnostic()?;

	{
		let mut state = bot_state.write().await;
		let start_ticket_message_state = state
			.entry::<StartTicketMessageState>()
			.or_insert_with(StartTicketMessageState::default);
		start_ticket_message_state.guilds.insert(prompt_id.clone(), guild_id);
	}

	tokio::spawn(expire_message_edit(bot_state, prompt_id));

	Ok(())
}

async fn expire_message_edit(bot_state: Arc<RwLock<TypeMap>>, prompt_id: String) {
	sleep(Duration::from_secs(3600)).await;
	let mut state = bot_state.write().await;
	let Some(edit_guilds) = state.get_mut::<StartTicketMessageState>() else {
		return;
	};
	edit_guilds.guilds.remove(&prompt_id);
}
