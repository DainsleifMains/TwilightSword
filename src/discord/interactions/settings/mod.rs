// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::bail;
use std::sync::Arc;
use tokio::sync::RwLock;
use twilight_http::client::Client;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use type_map::concurrent::TypeMap;

mod start_ticket_message;

pub async fn route_settings_modal(
	interaction: &InteractionCreate,
	modal_data: &ModalInteractionData,
	custom_id_path: &[String],
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let next_route = custom_id_path.get(1);

	match next_route.map(|route| route.as_str()) {
		Some("start_ticket_message") => {
			start_ticket_message::handle_start_ticket_message_modal(
				interaction,
				modal_data,
				custom_id_path,
				http_client,
				application_id,
				db_connection_pool,
				bot_state,
			)
			.await
		}
		_ => bail!(
			"Unexpected settings modal response encountered: {}\n{:?}",
			modal_data.custom_id,
			modal_data
		),
	}
}
