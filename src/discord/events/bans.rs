// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::{database_id_from_discord_id, BanAction, Guild};
use crate::schema::{ban_actions, guilds};
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

pub async fn handle_ban(
	event_audit_entry: &AuditLogEntry,
	http_client: Arc<Client>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> miette::Result<()> {
	let Some(guild_id) = event_audit_entry.guild_id else {
		return Ok(());
	};
	let Some(banning_user_id) = event_audit_entry.user_id else {
		bail!("Banning user not in ban audit data: {:?}", event_audit_entry);
	};
	let Some(banned_user_id) = event_audit_entry.target_id else {
		bail!("Banned user not in ban audit data: {:?}", event_audit_entry);
	};
	let banned_user_id: Id<UserMarker> = banned_user_id.cast();

	let guild = database_id_from_discord_id(guild_id.get());
	let banning_user = database_id_from_discord_id(banning_user_id.get());
	let banned_user = database_id_from_discord_id(banned_user_id.get());

	let action_time = event_audit_entry.id.timestamp();
	let MappedLocalTime::Single(action_time) = Utc.timestamp_millis_opt(action_time) else {
		bail!("Invalid timestamp provided with ban: {:?}", action_time);
	};

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;

	let db_guild_result: QueryResult<Option<Guild>> = guilds::table.find(guild).first(&mut db_connection).optional();
	let guild_data = match db_guild_result {
		Ok(Some(guild)) => guild,
		Ok(None) => return Ok(()),
		Err(error) => bail!(error),
	};

	let new_ban_action = BanAction {
		id: cuid2::create_id(),
		guild,
		banning_user,
		banned_user,
		added: true,
		action_time,
		reason: event_audit_entry.reason.clone().unwrap_or_default(),
	};

	diesel::insert_into(ban_actions::table)
		.values(new_ban_action)
		.execute(&mut db_connection)
		.into_diagnostic()?;

	if let Some(complain_channel_id) = guild_data.get_action_reason_complain_channel() {
		let complain_message = format!("{0}, {1} has been banned, but you didn't provide a reason. Please write a note about {1} as soon as you can.", banning_user_id.mention(), banned_user_id.mention());
		let mut allowed_mentions = AllowedMentions::default();
		allowed_mentions.users.push(banning_user_id);
		http_client
			.create_message(complain_channel_id)
			.content(&complain_message)
			.allowed_mentions(Some(&allowed_mentions))
			.await
			.into_diagnostic()?;
	}

	Ok(())
}

pub async fn handle_unban(
	event_audit_entry: &AuditLogEntry,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> miette::Result<()> {
	let Some(guild_id) = event_audit_entry.guild_id else {
		return Ok(());
	};
	let Some(banning_user) = event_audit_entry.user_id else {
		bail!("Unbanning user not in unban audit data: {:?}", event_audit_entry);
	};
	let Some(banned_user) = event_audit_entry.target_id else {
		bail!("Unbanned user not in unban audit data: {:?}", event_audit_entry);
	};

	let guild = database_id_from_discord_id(guild_id.get());
	let banning_user = database_id_from_discord_id(banning_user.get());
	let banned_user = database_id_from_discord_id(banned_user.get());

	let action_time = event_audit_entry.id.timestamp();
	let MappedLocalTime::Single(action_time) = Utc.timestamp_millis_opt(action_time) else {
		bail!("Invalid timestamp provided with unban: {:?}", action_time);
	};

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;

	let db_guild_result: QueryResult<Option<Guild>> = guilds::table.find(guild).first(&mut db_connection).optional();
	match db_guild_result {
		Ok(Some(_)) => (),
		Ok(None) => return Ok(()),
		Err(error) => bail!(error),
	}

	let new_unban_action = BanAction {
		id: cuid2::create_id(),
		guild,
		banning_user,
		banned_user,
		added: false,
		action_time,
		reason: String::new(),
	};

	diesel::insert_into(ban_actions::table)
		.values(new_unban_action)
		.execute(&mut db_connection)
		.into_diagnostic()?;

	Ok(())
}
