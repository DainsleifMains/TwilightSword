// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::utils::tickets::{staff_message, user_message, UserMessageAuthor};
use super::utils::timestamp::datetime_from_timestamp;
use crate::model::{database_id_from_discord_id, Ticket, TicketMessage};
use crate::schema::{ticket_messages, tickets};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::IntoDiagnostic;
use std::future::IntoFuture;
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
		.filter(
			tickets::staff_thread
				.eq(db_channel_id)
				.or(tickets::user_thread.eq(db_channel_id))
				.and(tickets::closed_at.is_null()),
		)
		.first(&mut db_connection)
		.optional()
		.into_diagnostic()?;
	let Some(ticket) = ticket else {
		return Ok(());
	};
	let message_from_staff = ticket.staff_thread == db_channel_id;

	let reply_message_id = message.reference.as_ref().and_then(|message_reference| {
		if message_reference.kind == MessageReferenceType::Default {
			message_reference.message_id
		} else {
			None
		}
	});

	let internal = if message_from_staff {
		if let Some(message_id) = reply_message_id {
			let db_message_id = database_id_from_discord_id(message_id.get());
			let message: Option<TicketMessage> = ticket_messages::table
				.filter(
					ticket_messages::staff_message
						.eq(db_message_id)
						.and(ticket_messages::user_message.is_not_null()),
				)
				.first(&mut db_connection)
				.optional()
				.into_diagnostic()?;
			message.is_none()
		} else {
			true
		}
	} else {
		false
	};

	let Some(message_time) = datetime_from_timestamp(&message.timestamp) else {
		return Ok(());
	};

	let staff_message_future = if internal {
		None
	} else {
		let Ok(staff_message_data) = staff_message(&message.author.name, &message.content, message.timestamp) else {
			return Ok(());
		};
		let staff_thread = ticket.get_staff_thread();

		let staff_message_create = staff_message_data.set_create_message_data(http_client.create_message(staff_thread));
		let staff_message_future = staff_message_create
			.embeds(&staff_message_data.embeds)
			.allowed_mentions(Some(&staff_message_data.allowed_mentions))
			.into_future();
		Some(staff_message_future)
	};

	let user_message_future = if internal {
		None
	} else {
		let user = ticket.get_with_user();
		let author = if message_from_staff {
			UserMessageAuthor::Staff
		} else {
			UserMessageAuthor::User(message.author.name.clone())
		};
		let Ok(user_message_data) = user_message(author, user, message_from_staff, &message.content, message.timestamp)
		else {
			return Ok(());
		};
		let user_thread = ticket.get_user_thread();
		let user_message_create = user_message_data.set_create_message_data(http_client.create_message(user_thread));
		let user_message_future = user_message_create.into_future();
		Some(user_message_future)
	};

	let (staff_message_result, user_message_result) = match (staff_message_future, user_message_future) {
		(Some(staff_future), Some(user_future)) => {
			let (staff, user) = tokio::join!(staff_future, user_future);
			(Some(staff), Some(user))
		}
		(Some(staff_future), None) => (Some(staff_future.await), None),
		(None, Some(user_future)) => (None, Some(user_future.await)),
		(None, None) => (None, None),
	};

	let staff_message = match staff_message_result {
		Some(result) => {
			let response = result.into_diagnostic()?;
			Some(response.model().await.into_diagnostic()?)
		}
		None => None,
	};

	let user_message = match user_message_result {
		Some(result) => {
			let response = result.into_diagnostic()?;
			Some(response.model().await.into_diagnostic()?)
		}
		None => None,
	};

	let staff_message_id = match staff_message {
		Some(message) => message.id,
		None => message.id,
	};

	let author = database_id_from_discord_id(message.author.id.get());
	let staff_message = database_id_from_discord_id(staff_message_id.get());
	let user_message = user_message.map(|message| database_id_from_discord_id(message.id.get()));
	let new_message = TicketMessage {
		id: cuid2::create_id(),
		ticket: ticket.id,
		author,
		send_time: message_time,
		body: message.content.clone(),
		staff_message,
		user_message,
	};
	diesel::insert_into(ticket_messages::table)
		.values(new_message)
		.execute(&mut db_connection)
		.into_diagnostic()?;

	Ok(())
}
