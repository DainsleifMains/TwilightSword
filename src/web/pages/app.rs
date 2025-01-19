// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos_meta::{provide_meta_context, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::StaticSegment;

#[component]
pub fn App() -> impl IntoView {
	provide_meta_context();

	view! {
		<Stylesheet href="/pkg/twilight-sword.css" />
		<Title text="Twilight Sword" />

		<Router>
			<Routes fallback=|| "Not found.".into_view()>
				<Route path=StaticSegment("") view=|| view! { <p>"Discord user ID: " <DiscordIdView /></p> } />
			</Routes>
		</Router>
	}
}

#[component]
pub fn DiscordIdView() -> impl IntoView {
	view! {
		<Await future=get_user_id() let:user_id>
			{match user_id {
				Ok(id) => id.to_string(),
				Err(_) => String::from(":(")
			}}
		</Await>
	}
}

#[server]
pub async fn get_user_id() -> Result<u64, ServerFnError> {
	use crate::web::session_key::DISCORD_USER;
	use crate::web::state::AppState;
	use leptos_axum::extract_with_state;
	use tower_sessions::Session;
	use twilight_model::id::marker::UserMarker;
	use twilight_model::id::Id;

	let state: AppState = expect_context();
	let session: Session = match extract_with_state(&state).await {
		Ok(session) => session,
		Err(error) => {
			tracing::error!(source = ?error, "Session extraction error");
			return Err(ServerFnError::ServerError(String::from("Session error")));
		}
	};
	let id: Option<Id<UserMarker>> = match session.get(DISCORD_USER).await {
		Ok(id) => id,
		Err(error) => {
			tracing::error!(source = ?error, "Session data error");
			return Err(ServerFnError::ServerError(String::from("Session error")));
		}
	};
	Ok(match id {
		Some(id) => id.get(),
		None => 0,
	})
}
