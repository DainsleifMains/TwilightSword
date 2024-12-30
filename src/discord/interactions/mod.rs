// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use twilight_http::client::Client;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use type_map::concurrent::TypeMap;

mod settings;
mod setup;

pub const MAX_INTERACTION_WAIT_TIME: Duration = Duration::from_secs(895);

pub async fn route_interaction(
	interaction: &InteractionCreate,
	interaction_data: &MessageComponentInteractionData,
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let custom_id_path: Vec<String> = interaction_data.custom_id.split('/').map(|s| s.to_string()).collect();

	match custom_id_path.first().map(|s| s.as_str()) {
		Some("setup") => {
			setup::route_setup_interaction(
				interaction,
				interaction_data,
				&custom_id_path,
				http_client,
				application_id,
				db_connection_pool,
				bot_state,
			)
			.await
		}
		_ => unimplemented!(),
	}
}

pub async fn route_modal_submit(
	interaction: &InteractionCreate,
	modal_data: &ModalInteractionData,
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let custom_id_path: Vec<String> = modal_data.custom_id.split('/').map(|s| s.to_string()).collect();

	match custom_id_path.first().map(|s| s.as_str()) {
		Some("settings") => {
			settings::route_settings_modal(
				interaction,
				modal_data,
				&custom_id_path,
				http_client,
				application_id,
				db_connection_pool,
				bot_state,
			)
			.await
		}
		_ => unimplemented!(),
	}
}
