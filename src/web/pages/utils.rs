// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct GuildData {
	pub name: String,
	pub icon_image_url: Option<String>,
}

#[server]
pub async fn get_guild_data(guild_id: Option<u64>) -> Result<Option<GuildData>, ServerFnError> {
	use crate::model::Guild;
	use crate::schema::guilds;
	use crate::web::state::AppState;
	use axum::extract::Host;
	use diesel::prelude::*;
	use leptos_axum::extract_with_state;
	use twilight_model::id::Id;

	let state = expect_context::<AppState>();

	let mut db_connection = state.db_connection_pool.get()?;

	let Host(host) = extract_with_state(&state).await?;
	let host_guild: Option<Guild> = guilds::table
		.filter(guilds::custom_host.eq(&host))
		.first(&mut db_connection)
		.optional()?;
	let host_guild_id = host_guild.as_ref().map(|guild| guild.get_guild_id());

	let guild_id = match guild_id {
		Some(id) => {
			if host_guild_id.is_some() {
				None
			} else {
				Some(Id::new(id))
			}
		}
		None => host_guild_id,
	};

	let Some(guild_id) = guild_id else {
		return Ok(None);
	};

	let discord_client = &state.discord_client;
	let guild_data = discord_client.guild(guild_id).await?;
	let guild_data = guild_data.model().await?;

	let name = guild_data.name;
	let icon_image_url = guild_data
		.icon
		.map(|icon_hash| format!("https://cdn.discordapp.com/icons/{}/{}.png", guild_id, icon_hash));

	Ok(Some(GuildData { name, icon_image_url }))
}
