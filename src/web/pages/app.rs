// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::dashboard::Dashboard;
use super::errors::error::Error;
use super::errors::not_found::NotFound;
use super::header::PageHeader;
use super::utils::get_guild_data;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Stylesheet, Title};
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::hooks::use_params;
use leptos_router::nested_router::Outlet;
use leptos_router::params::Params;
use leptos_router::path;

#[component]
pub fn App() -> impl IntoView {
	provide_meta_context();

	view! {
		<Stylesheet href="/pkg/twilight-sword.css" />
		<Title text="Twilight Sword" />

		<Router>
			<Routes fallback=|| "Not found.">
				<ParentRoute path=path!("/:guild?") view=MainPage>
					<Route path=path!("/") view=Dashboard />
				</ParentRoute>
			</Routes>
		</Router>
	}
}

#[derive(Params, PartialEq)]
struct GuildParam {
	guild: Option<u64>,
}

#[component]
fn MainPage() -> impl IntoView {
	let params = use_params::<GuildParam>();

	let guild_id = params.read().as_ref().ok().and_then(|params| params.guild);

	view! {
		<Await future=get_guild_data(guild_id) let:data>
			{
				match data {
					Ok(Some(data)) => view! {
						<PageHeader guild_data={data.clone()} />
						<main>
							<Outlet />
						</main>
					}.into_any(),
					Ok(None) => view! {
						<NotFound />
					}.into_any(),
					Err(_) => view! {
						<Error />
					}.into_any()
				}
			}
		</Await>
	}
}
