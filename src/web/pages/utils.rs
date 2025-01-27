// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use leptos::prelude::*;
use leptos_router::params::Params;
use serde::{Deserialize, Serialize};

#[derive(Params, PartialEq)]
pub struct GuildParam {
	pub guild: Option<u64>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct GuildData {
	pub name: String,
	pub icon_image_url: Option<String>,
}

#[server]
pub async fn get_guild_data(guild_id: Option<u64>) -> Result<Option<GuildData>, ServerFnError> {
	use super::server_utils::get_guild_id_from_request;
	use crate::web::state::AppState;

	let guild_id = get_guild_id_from_request(guild_id).await?;

	let Some(guild_id) = guild_id else {
		return Ok(None);
	};

	let state = expect_context::<AppState>();
	let discord_client = &state.discord_client;
	let guild_data = discord_client.guild(guild_id).await?;
	let guild_data = guild_data.model().await?;

	let name = guild_data.name;
	let icon_image_url = guild_data
		.icon
		.map(|icon_hash| format!("https://cdn.discordapp.com/icons/{}/{}.png", guild_id, icon_hash));

	Ok(Some(GuildData { name, icon_image_url }))
}
