// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use chrono::{DateTime, Utc};
use leptos::prelude::*;
use leptos_router::params::Params;
use reactive_stores::Store;
use serde::{Deserialize, Serialize};

#[derive(Debug, Params, PartialEq)]
pub struct GuildParam {
	pub guild: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Params, PartialEq)]
pub struct TicketParams {
	pub guild: Option<u64>,
	pub ticket: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Store)]
pub struct TicketData {
	pub title: String,
	pub category_name: String,
	#[store(key: String = |message| message.id.clone())]
	pub messages: Vec<TicketMessage>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TicketMessage {
	pub id: String,
	pub author_name: String,
	pub send_time: DateTime<Utc>,
	pub internal: bool,
	pub body: String,
}

/// Makes a URL to the view for a ticket
pub fn make_ticket_url(guild_id: Option<u64>, ticket_id: &str) -> String {
	match guild_id {
		Some(id) => format!("/{}/ticket/{}", id, ticket_id),
		None => format!("/ticket/{}", ticket_id),
	}
}
