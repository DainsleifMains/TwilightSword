// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::commands::{command_definitions, route_command};
use super::interactions::route_interaction;
use crate::config::ConfigDocument;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::IntoDiagnostic;
use std::sync::Arc;
use tokio::sync::RwLock;
use twilight_cache_inmemory::{DefaultInMemoryCache, ResourceType};
use twilight_gateway::{EventTypeFlags, Intents, Shard, ShardId, StreamExt};
use twilight_http::client::Client;
use twilight_model::application::interaction::InteractionData;
use twilight_model::gateway::event::Event;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use type_map::concurrent::TypeMap;

pub async fn run_bot(
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	config: Arc<ConfigDocument>,
) -> miette::Result<()> {
	let intents = Intents::empty();

	let mut shard = Shard::new(ShardId::ONE, config.discord_token.clone(), intents);

	let http_client = Arc::new(Client::new(config.discord_token.clone()));

	let cache = DefaultInMemoryCache::builder()
		.resource_types(ResourceType::all())
		.build();

	let application_id = {
		let application_response = http_client.current_user_application().await.into_diagnostic()?;
		application_response.model().await.into_diagnostic()?.id
	};

	{
		let interaction_client = http_client.interaction(application_id);
		let commands = command_definitions();
		interaction_client
			.set_global_commands(&commands)
			.await
			.into_diagnostic()?;
	}

	let bot_state = Arc::new(RwLock::new(TypeMap::new()));

	while let Some(event) = shard.next_event(EventTypeFlags::all()).await {
		let event = match event {
			Ok(event) => event,
			Err(error) => {
				tracing::warn!(source = ?error, "error receiving event");
				continue;
			}
		};
		cache.update(&event);

		tokio::spawn(handle_event(
			event,
			Arc::clone(&http_client),
			application_id,
			db_connection_pool.clone(),
			Arc::clone(&bot_state),
		));
	}

	Ok(())
}

async fn handle_event(
	event: Event,
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	match event {
		Event::InteractionCreate(interaction) => match &interaction.data {
			Some(InteractionData::ApplicationCommand(command_data)) => {
				route_command(
					&interaction,
					command_data,
					http_client,
					application_id,
					db_connection_pool,
					bot_state,
				)
				.await?;
			}
			Some(InteractionData::MessageComponent(interaction_data)) => {
				route_interaction(
					&interaction,
					interaction_data,
					http_client,
					application_id,
					db_connection_pool,
					bot_state,
				)
				.await?;
			}
			_ => (),
		},
		Event::Ready(_) => {
			tracing::debug!("Gateway is ready");
		}
		_ => (),
	}
	Ok(())
}