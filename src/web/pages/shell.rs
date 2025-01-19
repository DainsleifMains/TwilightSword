// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::app::App;
use leptos::prelude::*;
use leptos_meta::MetaTags;

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
