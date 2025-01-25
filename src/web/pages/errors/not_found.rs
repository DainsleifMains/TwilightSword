// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use leptos::prelude::*;

#[component]
pub fn NotFound() -> impl IntoView {
	view! {
		<main id="not_found_page">
			<h1>"Not found!"</h1>
			<p>"The content you were looking for is not here."</p>
		</main>
	}
}
