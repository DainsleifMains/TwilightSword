// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::utils::GuildData;
use leptos::prelude::*;

#[component]
pub fn PageHeader(guild_data: GuildData) -> impl IntoView {
	view! {
		<div id="header">
			<div>
				{
					guild_data
						.icon_image_url
						.as_ref()
						.map(|url| view! {
							<img id="header_guild_icon" src={url.clone()} alt="Server Icon" />
						})
				}
			</div>
			<h1 id="header_guild_name">{guild_data.name.clone()}</h1>
		</div>
	}
}
