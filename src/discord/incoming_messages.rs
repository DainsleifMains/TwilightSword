// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::utils::timestamp::datetime_from_timestamp;
use crate::model::{database_id_from_discord_id, Ticket, TicketMessage};
use crate::schema::{ticket_messages, tickets};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::IntoDiagnostic;
use twilight_http::client::Client;
use twilight_model::channel::message::{Message, MessageReferenceType};

pub async fn handle_message(
	message: &Message,
	http_client: &Client,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> miette::Result<()> {
	let bot_user_response = http_client.current_user().await.into_diagnostic()?;
	let bot_user = bot_user_response.model().await.into_diagnostic()?;
	if message.author.id == bot_user.id {
		return Ok(());
	}

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;

	let db_channel_id = database_id_from_discord_id(message.channel_id.get());
	let ticket: Option<Ticket> = tickets::table
		.filter(tickets::staff_thread.eq(db_channel_id))
		.first(&mut db_connection)
		.optional()
		.into_diagnostic()?;
	let Some(ticket) = ticket else {
		return Ok(());
	};

	let reply_message_id = message.reference.as_ref().and_then(|message_reference| {
		if message_reference.kind == MessageReferenceType::Default {
			message_reference.message_id
		} else {
			None
		}
	});

	let internal = if let Some(message_id) = reply_message_id {
		let db_message_id = database_id_from_discord_id(message_id.get());
		let message: Option<TicketMessage> = ticket_messages::table
			.filter(
				ticket_messages::staff_message
					.eq(db_message_id)
					.and(ticket_messages::internal.eq(false)),
			)
			.first(&mut db_connection)
			.optional()
			.into_diagnostic()?;
		message.is_none()
	} else {
		true
	};

	let Some(message_time) = datetime_from_timestamp(&message.timestamp) else {
		return Ok(());
	};

	let author = database_id_from_discord_id(message.author.id.get());
	let staff_message = database_id_from_discord_id(message.id.get());
	let new_message = TicketMessage {
		id: cuid2::create_id(),
		ticket: ticket.id,
		author,
		send_time: message_time,
		internal,
		body: message.content.clone(),
		staff_message,
	};
	diesel::insert_into(ticket_messages::table)
		.values(new_message)
		.execute(&mut db_connection)
		.into_diagnostic()?;

	Ok(())
}
