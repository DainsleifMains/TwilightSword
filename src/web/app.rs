// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::StaticSegment;

pub fn shell(options: LeptosOptions) -> impl IntoView {
	view! {
		<!DOCTYPE html>
		<html>
			<head>
				<meta charset="utf-8" />
				<HydrationScripts options />
				<MetaTags />
			</head>
			<body>
				<App />
			</body>
		</html>
	}
}

#[component]
pub fn App() -> impl IntoView {
	provide_meta_context();

	view! {
		<Stylesheet href="/pkg/twilight-sword.css" />
		<Title text="Twilight Sword" />

		<Router>
			<Routes fallback=|| "Not found.".into_view()>
				<Route path=StaticSegment("") view=|| view! { <p>"TODO write something here"</p> } />
			</Routes>
		</Router>
	}
}
