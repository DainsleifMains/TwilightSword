// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::Guild;
use crate::schema::guilds;
use crate::web::session_key::DISCORD_USER;
use crate::web::state::AppState;
use axum::extract::Host;
use diesel::prelude::*;
use leptos::prelude::*;
use leptos_axum::extract_with_state;
use tower_sessions::session::Session;
use twilight_model::id::marker::{GuildMarker, UserMarker};
use twilight_model::id::Id;

/// Gets the guild ID for a request.
/// Must be used from a server function; relies on extracting request data.
pub async fn get_guild_id_from_request(client_guild_id: Option<u64>) -> Result<Option<Id<GuildMarker>>, ServerFnError> {
	let state: AppState = expect_context();

	let mut db_connection = state.db_connection_pool.get()?;

	let Host(host) = extract_with_state(&state).await?;
	let host_guild: Option<Guild> = guilds::table
		.filter(guilds::custom_host.eq(&host))
		.first(&mut db_connection)
		.optional()?;
	let host_guild_id = host_guild.as_ref().map(|guild| guild.get_guild_id());

	let guild_id = match client_guild_id {
		Some(id) => {
			if host_guild_id.is_some() {
				None
			} else {
				Some(Id::new(id))
			}
		}
		None => host_guild_id,
	};

	Ok(guild_id)
}

/// Gets the user ID for a request.
/// Must be used from a server function; relies on extracting request data.
pub async fn get_user_id_from_request() -> Result<Option<Id<UserMarker>>, ServerFnError> {
	let state: AppState = expect_context();
	let session: Session = extract_with_state(&state).await?;
	let user_id: Option<Id<UserMarker>> = session.get(DISCORD_USER).await?;
	Ok(user_id)
}
