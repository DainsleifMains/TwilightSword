// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::dashboard::Dashboard;
use super::header::PageHeader;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Stylesheet, Title};
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::nested_router::Outlet;
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

#[component]
fn MainPage() -> impl IntoView {
	view! {
		<PageHeader />
		<main>
			<Outlet />
		</main>
	}
}
