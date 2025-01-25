// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use leptos::prelude::*;
use leptos_router::hooks::use_params;
use leptos_router::params::Params;
use serde::{Deserialize, Serialize};

#[derive(Params, PartialEq)]
struct GuildParam {
	guild: Option<u64>,
}

#[component]
pub fn PageHeader() -> impl IntoView {
	let params = use_params::<GuildParam>();

	let guild_id = params.read().as_ref().ok().and_then(|params| params.guild);

	view! {
		<div id="header">
			<Await future=guild_header_data(guild_id) let:guild_data>
				{
					guild_data.as_ref().ok().flatten().map(|data| view! {
						<div>
							{
								data
									.icon_image_url
									.as_ref()
									.map(|url| view! {
										<img id="header_guild_icon" src={url.clone()} alt="Server Icon" />
									})
							}
						</div>
						<h1 id="header_guild_name">{data.name.clone()}</h1>
					})
				}
			</Await>
		</div>
	}
}

#[derive(Deserialize, Serialize)]
pub struct GuildData {
	name: String,
	icon_image_url: Option<String>,
}

#[server]
async fn guild_header_data(guild_id: Option<u64>) -> Result<Option<GuildData>, ServerFnError> {
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
