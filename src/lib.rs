// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(feature = "ssr")]
pub mod config;
#[cfg(feature = "ssr")]
pub mod database;
#[cfg(feature = "ssr")]
pub mod discord;
#[cfg(feature = "ssr")]
pub mod model;
#[cfg(feature = "ssr")]
pub mod schema;
pub mod web;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
	use crate::web::app::App;
	console_error_panic_hook::set_once();
	leptos::mount::hydrate_body(App);
}
