// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use leptos::prelude::*;

#[component]
pub fn Error() -> impl IntoView {
	view! {
		<h1>"Error!"</h1>
		<p>"An error occurred handling this request."</p>
	}
}
