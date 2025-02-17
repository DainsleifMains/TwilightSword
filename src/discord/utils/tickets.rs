// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fmt;
use twilight_http::request::channel::message::create_message::CreateMessage;
use twilight_mention::fmt::Mention;
use twilight_model::channel::message::embed::Embed;
use twilight_model::channel::message::AllowedMentions;
use twilight_model::http::interaction::InteractionResponseData;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;
use twilight_model::util::datetime::Timestamp;
use twilight_util::builder::embed::{EmbedAuthorBuilder, EmbedBuilder};
use twilight_util::builder::InteractionResponseDataBuilder;
use twilight_validate::embed::EmbedValidationError;

pub const MAX_TICKET_TITLE_LENGTH: u16 = 60;

/// Indicates the author of a message being sent on the user end of the ticket
pub enum UserMessageAuthor {
	User(String),
	Staff,
}

impl fmt::Display for UserMessageAuthor {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::User(name) => write!(f, "{}", name),
			Self::Staff => write!(f, "Staff"),
		}
	}
}

/// Contains data necessary to post a ticket message
pub struct TicketMessageData {
	pub content: Option<String>,
	pub embeds: Vec<Embed>,
	pub allowed_mentions: AllowedMentions,
}

impl TicketMessageData {
	/// Adds all of the ticket message data to a [CreateMessage] builder
	pub fn set_create_message_data<'a>(&'a self, mut create_message: CreateMessage<'a>) -> CreateMessage<'a> {
		if let Some(content) = &self.content {
			create_message = create_message.content(content);
		}
		create_message
			.embeds(&self.embeds)
			.allowed_mentions(Some(&self.allowed_mentions))
	}
}

impl From<TicketMessageData> for InteractionResponseData {
	fn from(data: TicketMessageData) -> Self {
		let mut response = InteractionResponseDataBuilder::new();
		if let Some(content) = &data.content {
			response = response.content(content)
		}
		response
			.embeds(data.embeds)
			.allowed_mentions(data.allowed_mentions)
			.build()
	}
}

/// Generates the message data for sending a ticket message to the staff end of the ticket
pub fn staff_message(
	author_name: &str,
	message: &str,
	timestamp: Timestamp,
) -> Result<TicketMessageData, EmbedValidationError> {
	let author = EmbedAuthorBuilder::new(author_name).build();
	let embed = EmbedBuilder::new()
		.description(message)
		.author(author)
		.timestamp(timestamp)
		.validate()?
		.build();
	Ok(TicketMessageData {
		content: None,
		embeds: vec![embed],
		allowed_mentions: AllowedMentions::default(),
	})
}

/// Generates the message data for sending a ticket message to the user end of the ticket
pub fn user_message(
	author: UserMessageAuthor,
	ticket_with_user: Id<UserMarker>,
	include_ping: bool,
	message: &str,
	timestamp: Timestamp,
) -> Result<TicketMessageData, EmbedValidationError> {
	let author = EmbedAuthorBuilder::new(author.to_string()).build();
	let embed = EmbedBuilder::new()
		.description(message)
		.author(author)
		.timestamp(timestamp)
		.validate()?
		.build();
	let content = if include_ping {
		Some(format!("{}", ticket_with_user.mention()))
	} else {
		None
	};
	let mut allowed_mentions = AllowedMentions::default();
	allowed_mentions.users.push(ticket_with_user);
	Ok(TicketMessageData {
		content,
		embeds: vec![embed],
		allowed_mentions,
	})
}
