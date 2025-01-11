// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::{database_id_from_discord_id, Guild, TimeoutAction};
use crate::schema::{guilds, timeout_actions};
use chrono::offset::MappedLocalTime;
use chrono::{TimeZone, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{bail, IntoDiagnostic};
use twilight_http::client::Client;
use twilight_mention::fmt::Mention;
use twilight_model::channel::message::AllowedMentions;
use twilight_model::guild::audit_log::AuditLogEntry;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;
use twilight_model::util::datetime::Timestamp;
use twilight_util::snowflake::Snowflake;

pub async fn handle_timeout_update(
	event_audit_entry: &AuditLogEntry,
	expires_at: &Option<Timestamp>,
	http_client: &Client,
	db_connection_pool: &Pool<ConnectionManager<PgConnection>>,
) -> miette::Result<()> {
	let Some(guild_id) = event_audit_entry.guild_id else {
		return Ok(());
	};
	let Some(performing_user_id) = event_audit_entry.user_id else {
		bail!("Timeout action has no performing user: {:?}", event_audit_entry);
	};
	let Some(target_user_id) = event_audit_entry.target_id else {
		bail!("Timeout action has no timed out user: {:?}", event_audit_entry);
	};
	let target_user_id: Id<UserMarker> = target_user_id.cast();

	let guild = database_id_from_discord_id(guild_id.get());
	let performing_user = database_id_from_discord_id(performing_user_id.get());
	let target_user = database_id_from_discord_id(target_user_id.get());
	let reason = event_audit_entry.reason.clone().unwrap_or_default();

	let action_time = event_audit_entry.id.timestamp();
	let MappedLocalTime::Single(action_time) = Utc.timestamp_millis_opt(action_time) else {
		bail!("Timeout action has invalid timestamp: {:?}", event_audit_entry);
	};

	let timeout_until = expires_at.map(|ts| Utc.timestamp_micros(ts.as_micros()));
	let timeout_until = match timeout_until {
		Some(expiry) => {
			let MappedLocalTime::Single(expiry) = expiry else {
				bail!("Invalid timeout expiration: {:?}", expiry);
			};
			Some(expiry)
		}
		None => None,
	};

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;

	let db_guild_result: QueryResult<Option<Guild>> = guilds::table.find(guild).first(&mut db_connection).optional();
	let guild_data = match db_guild_result {
		Ok(Some(guild)) => guild,
		Ok(None) => return Ok(()),
		Err(error) => bail!(error),
	};

	let new_timeout_action = TimeoutAction {
		id: cuid2::create_id(),
		guild,
		performing_user,
		target_user,
		action_time,
		timeout_until,
		reason,
	};

	diesel::insert_into(timeout_actions::table)
		.values(new_timeout_action)
		.execute(&mut db_connection)
		.into_diagnostic()?;

	if timeout_until.is_some() {
		if let Some(complain_channel_id) = guild_data.get_action_reason_complain_channel() {
			let complain_message = format!("{0}, {1} has been timed out (or the timeout duration was updated), but you didn't provide a reason. Please write a note about {1} as soon as you can.", performing_user_id.mention(), target_user_id.mention());
			let mut allowed_mentions = AllowedMentions::default();
			allowed_mentions.users.push(performing_user_id);
			http_client
				.create_message(complain_channel_id)
				.content(&complain_message)
				.allowed_mentions(Some(&allowed_mentions))
				.await
				.into_diagnostic()?;
		}
	}

	Ok(())
}
