// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::sync::Arc;
use twilight_http::client::Client;
use twilight_model::guild::audit_log::{AuditLogChange, AuditLogEntry, AuditLogEventType};

mod automod;
mod bans;
mod kicks;
mod timeouts;

pub async fn route_events(
	event_audit_entry: &AuditLogEntry,
	http_client: Arc<Client>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> miette::Result<()> {
	match event_audit_entry.action_type {
		AuditLogEventType::AutoModerationBlockMessage => {
			automod::handle_block(event_audit_entry, db_connection_pool).await?
		}
		AuditLogEventType::AutoModerationUserCommunicationDisabled => {
			automod::handle_timeout(event_audit_entry, db_connection_pool).await?
		}
		AuditLogEventType::MemberBanAdd => bans::handle_ban(event_audit_entry, http_client, db_connection_pool).await?,
		AuditLogEventType::MemberBanRemove => bans::handle_unban(event_audit_entry, db_connection_pool).await?,
		AuditLogEventType::MemberKick => kicks::handle_kick(event_audit_entry, http_client, db_connection_pool).await?,
		AuditLogEventType::MemberUpdate => {
			for change in event_audit_entry.changes.iter() {
				if let AuditLogChange::CommunicationDisabledUntil { new, .. } = change {
					timeouts::handle_timeout_update(event_audit_entry, new, &http_client, &db_connection_pool).await?
				}
			}
		}
		_ => (),
	}

	Ok(())
}
