// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::{database_id_from_discord_id, Ticket};
use crate::schema::tickets;
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::IntoDiagnostic;
use std::future::IntoFuture;
use twilight_http::client::Client;
use twilight_http::request::AuditLogReason;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use twilight_util::builder::command::CommandBuilder;
use twilight_util::builder::InteractionResponseDataBuilder;

pub fn command_definition() -> Command {
	CommandBuilder::new("close", "Close a ticket", CommandType::ChatInput)
		.contexts([InteractionContextType::Guild])
		.build()
}

pub async fn handle_command(
	interaction: &InteractionCreate,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
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

	let close_time = Utc::now();
	let staff_thread_id = ticket.get_staff_thread();
	let user_thread_id = ticket.get_user_thread();

	diesel::update(tickets::table)
		.filter(tickets::id.eq(&ticket.id))
		.set(tickets::closed_at.eq(Some(close_time)))
		.execute(&mut db_connection)
		.into_diagnostic()?;

	let response = InteractionResponseDataBuilder::new()
		.content("This ticket has been closed.")
		.build();
	let response = InteractionResponse {
		kind: InteractionResponseType::ChannelMessageWithSource,
		data: Some(response),
	};
	let response_future = interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.into_future();

	let staff_thread_future = http_client
		.update_thread(staff_thread_id)
		.locked(true)
		.reason("Closed ticket")
		.into_future();
	let user_thread_future = http_client
		.update_thread(user_thread_id)
		.locked(true)
		.reason("Closed ticket")
		.into_future();
	let (response_result, staff_thread_result, user_thread_result) =
		tokio::join!(response_future, staff_thread_future, user_thread_future);
	response_result.into_diagnostic()?;
	staff_thread_result.into_diagnostic()?;
	user_thread_result.into_diagnostic()?;

	Ok(())
}
