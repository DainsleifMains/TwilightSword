// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::{database_id_from_discord_id, Guild, KickAction};
use crate::schema::{guilds, kick_actions};
use chrono::offset::MappedLocalTime;
use chrono::{TimeZone, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{bail, IntoDiagnostic};
use std::sync::Arc;
use twilight_http::client::Client;
use twilight_mention::fmt::Mention;
use twilight_model::channel::message::AllowedMentions;
use twilight_model::guild::audit_log::AuditLogEntry;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;
use twilight_util::snowflake::Snowflake;

pub async fn handle_kick(
	event_audit_entry: &AuditLogEntry,
	http_client: Arc<Client>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> miette::Result<()> {
	let Some(guild_id) = event_audit_entry.guild_id else {
		return Ok(());
	};
	let Some(kicking_user_id) = event_audit_entry.user_id else {
		bail!("Kick data doesn't contain kicking user: {:?}", event_audit_entry);
	};
	let Some(kicked_user_id) = event_audit_entry.target_id else {
		bail!("Kick data doesn't contain kicked user: {:?}", event_audit_entry);
	};
	let kicked_user_id: Id<UserMarker> = kicked_user_id.cast();

	let guild = database_id_from_discord_id(guild_id.get());
	let kicking_user = database_id_from_discord_id(kicking_user_id.get());
	let kicked_user = database_id_from_discord_id(kicked_user_id.get());
	let reason = event_audit_entry.reason.clone().unwrap_or_default();

	let action_time = event_audit_entry.id.timestamp();
	let MappedLocalTime::Single(action_time) = Utc.timestamp_millis_opt(action_time) else {
		bail!("Invalid timestamp with kick action: {:?}", event_audit_entry);
	};

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;

	let db_guild_result: QueryResult<Option<Guild>> = guilds::table.find(guild).first(&mut db_connection).optional();
	let guild_data = match db_guild_result {
		Ok(Some(guild)) => guild,
		Ok(None) => return Ok(()),
		Err(error) => bail!(error),
	};

	let new_kick_action = KickAction {
		id: cuid2::create_id(),
		guild,
		kicking_user,
		kicked_user,
		action_time,
		reason,
	};

	diesel::insert_into(kick_actions::table)
		.values(new_kick_action)
		.execute(&mut db_connection)
		.into_diagnostic()?;

	if let Some(complain_channel_id) = guild_data.get_action_reason_complain_channel() {
		let complain_message = format!("{0}, {1} has been kicked, but you didn't provide a reason. Please write a note about {1} as soon as you can.", kicking_user_id.mention(), kicked_user_id.mention());
		let mut allowed_mentions = AllowedMentions::default();
		allowed_mentions.users.push(kicking_user_id);
		http_client
			.create_message(complain_channel_id)
			.content(&complain_message)
			.allowed_mentions(Some(&allowed_mentions))
			.await
			.into_diagnostic()?;
	}

	Ok(())
}
