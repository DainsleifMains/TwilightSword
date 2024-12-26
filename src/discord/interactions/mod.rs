use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::sync::Arc;
use tokio::sync::RwLock;
use twilight_http::client::Client;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use type_map::concurrent::TypeMap;

mod setup;

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
				custom_id_path,
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
