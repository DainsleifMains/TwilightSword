// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::{database_id_from_discord_id, AutomodAction, AutomodActionType, Guild};
use crate::schema::{automod_actions, guilds};
use chrono::offset::MappedLocalTime;
use chrono::{TimeZone, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{bail, IntoDiagnostic};
use twilight_model::guild::audit_log::AuditLogEntry;
use twilight_util::snowflake::Snowflake;

pub async fn handle_block(
	event_audit_entry: &AuditLogEntry,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> miette::Result<()> {
	let Some(guild_id) = event_audit_entry.guild_id else {
		return Ok(());
	};
	let Some(target_user_id) = event_audit_entry.user_id else {
		return Ok(());
	};
	let Some(auto_mod_action_data) = &event_audit_entry.options else {
		return Ok(());
	};
	let rule_name = auto_mod_action_data
		.auto_moderation_rule_name
		.clone()
		.unwrap_or_default();

	let action_time = event_audit_entry.id.timestamp();
	let MappedLocalTime::Single(action_time) = Utc.timestamp_millis_opt(action_time) else {
		bail!("Invalid timestamp received for automod block: {:?}", action_time);
	};

	let guild = database_id_from_discord_id(guild_id.get());
	let target_user = database_id_from_discord_id(target_user_id.get());
	let reason = event_audit_entry.reason.clone().unwrap_or_default();

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;

	let db_guild_result: QueryResult<Option<Guild>> = guilds::table.find(guild).first(&mut db_connection).optional();
	match db_guild_result {
		Ok(Some(_)) => guild,
		Ok(None) => return Ok(()),
		Err(error) => bail!(error),
	};

	let new_automod_action = AutomodAction {
		id: cuid2::create_id(),
		guild,
		target_user,
		action_type: AutomodActionType::Block,
		action_time,
		reason,
		rule_name,
	};

	diesel::insert_into(automod_actions::table)
		.values(new_automod_action)
		.execute(&mut db_connection)
		.into_diagnostic()?;

	Ok(())
}

pub async fn handle_timeout(
	event_audit_entry: &AuditLogEntry,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> miette::Result<()> {
	let Some(guild_id) = event_audit_entry.guild_id else {
		return Ok(());
	};
	let Some(target_user_id) = event_audit_entry.user_id else {
		return Ok(());
	};
	let Some(auto_mod_action_data) = &event_audit_entry.options else {
		return Ok(());
	};
	let rule_name = auto_mod_action_data
		.auto_moderation_rule_name
		.clone()
		.unwrap_or_default();

	let action_time = event_audit_entry.id.timestamp();
	let MappedLocalTime::Single(action_time) = Utc.timestamp_millis_opt(action_time) else {
		bail!("Invalid timestamp received for automod timeout: {:?}", action_time);
	};

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;

	let new_automod_action = AutomodAction {
		id: cuid2::create_id(),
		guild: database_id_from_discord_id(guild_id.get()),
		target_user: database_id_from_discord_id(target_user_id.get()),
		action_type: AutomodActionType::DisableCommunication,
		action_time,
		reason: event_audit_entry.reason.clone().unwrap_or_default(),
		rule_name,
	};

	diesel::insert_into(automod_actions::table)
		.values(new_automod_action)
		.execute(&mut db_connection)
		.into_diagnostic()?;

	Ok(())
}
