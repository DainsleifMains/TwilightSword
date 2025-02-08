// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::commands::{command_definitions, route_command};
use super::events::route_events;
use super::interactions::{route_interaction, route_modal_submit};
use crate::config::ConfigData;
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

pub fn set_up_client(config: &ConfigData) -> Arc<Client> {
	Arc::new(Client::new(config.discord.bot_token.clone()))
}

pub async fn run_bot(
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	config: Arc<ConfigData>,
	http_client: Arc<Client>,
) -> miette::Result<()> {
	let intents = Intents::GUILD_MODERATION | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;

	let mut shard = Shard::new(ShardId::ONE, config.discord.bot_token.clone(), intents);

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
) {
	let event_result = handle_event_route(event, &http_client, application_id, db_connection_pool, bot_state).await;
	if let Err(error) = event_result {
		tracing::error!(source = ?error, "An error occurred handling a gateway event");
	}
}

async fn handle_event_route(
	event: Event,
	http_client: &Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	tracing::debug!("Incoming gateway message: {:?}", event);
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
			Some(InteractionData::ModalSubmit(modal_data)) => {
				route_modal_submit(
					&interaction,
					modal_data,
					http_client,
					application_id,
					db_connection_pool,
					bot_state,
				)
				.await?
			}
			_ => (),
		},
		Event::GuildAuditLogEntryCreate(event_audit_data) => {
			route_events(&event_audit_data.0, http_client, db_connection_pool).await?
		}
		Event::Ready(_) => {
			tracing::info!("Discord gateway is ready");
		}
		_ => (),
	}
	Ok(())
}
