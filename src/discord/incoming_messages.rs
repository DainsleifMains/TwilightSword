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
use twilight_model::channel::message::Message;

pub async fn handle_message(
	message: &Message,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> miette::Result<()> {
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

	let Some(message_time) = datetime_from_timestamp(&message.timestamp) else {
		return Ok(());
	};

	let author = database_id_from_discord_id(message.author.id.get());
	let new_message = TicketMessage {
		id: cuid2::create_id(),
		ticket: ticket.id,
		author,
		send_time: message_time,
		internal: true,
		body: message.content.clone(),
	};
	diesel::insert_into(ticket_messages::table)
		.values(new_message)
		.execute(&mut db_connection)
		.into_diagnostic()?;

	Ok(())
}
