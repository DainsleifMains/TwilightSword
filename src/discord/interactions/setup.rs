// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::state::setup::{SetupState, set_up_components};
use crate::model::{Guild, database_id_from_discord_id};
use crate::schema::guilds;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::{DatabaseErrorKind, Error as DbError};
use miette::{IntoDiagnostic, bail};
use std::sync::Arc;
use tokio::sync::RwLock;
use twilight_http::client::Client;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::{ApplicationMarker, RoleMarker};
use twilight_util::builder::InteractionResponseDataBuilder;
use type_map::concurrent::TypeMap;

const SETUP_EXPIRED_MESSAGE: &str = "Setup expired. Run `/setup` to set up again.";

pub async fn route_setup_interaction(
	interaction: &InteractionCreate,
	interaction_data: &MessageComponentInteractionData,
	custom_id_path: &[String],
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let Some(setup_id) = custom_id_path.get(1) else {
		bail!("Interaction ID not in setup ID route");
	};
	match custom_id_path.get(2).map(|s| s.as_str()) {
		Some("admin_role") => {
			handle_admin_role_update(
				interaction,
				interaction_data,
				setup_id,
				http_client,
				application_id,
				bot_state,
			)
			.await
		}
		Some("staff_role") => {
			handle_staff_role_update(
				interaction,
				interaction_data,
				setup_id,
				http_client,
				application_id,
				bot_state,
			)
			.await
		}
		Some("confirm") => {
			handle_confirm(
				interaction,
				setup_id,
				http_client,
				application_id,
				db_connection_pool,
				bot_state,
			)
			.await
		}
		Some("cancel") => handle_cancel(interaction, setup_id, http_client, application_id, bot_state).await,
		_ => bail!(
			"Unexpected setup interaction encountered: {}\n{:?}",
			interaction_data.custom_id,
			interaction_data
		),
	}
}

async fn handle_admin_role_update(
	interaction: &InteractionCreate,
	interaction_data: &MessageComponentInteractionData,
	setup_id: &str,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let mut state = bot_state.write().await;
	let Some(setup_state) = state.get_mut::<SetupState>() else {
		return Ok(());
	};
	let interaction_client = http_client.interaction(application_id);
	let Some(interaction_state) = setup_state.states.get_mut(setup_id) else {
		let response = InteractionResponseDataBuilder::new()
			.content(SETUP_EXPIRED_MESSAGE)
			.components(Vec::new())
			.build();
		let response = InteractionResponse {
			kind: InteractionResponseType::UpdateMessage,
			data: Some(response),
		};
		interaction_client
			.create_response(interaction.id, &interaction.token, &response)
			.await
			.into_diagnostic()?;
		return Ok(());
	};

	let role = match interaction_data.values.first() {
		Some(role_str) => {
			let role_id: u64 = role_str.parse().into_diagnostic()?;
			let role: Id<RoleMarker> = Id::new(role_id);
			Some(role)
		}
		None => None,
	};
	interaction_state.admin_role = role;

	let updated_components = set_up_components(
		setup_id,
		interaction_state.admin_role.is_none() || interaction_state.staff_role.is_none(),
	);
	let response = InteractionResponseDataBuilder::new()
		.components(updated_components)
		.build();
	let response = InteractionResponse {
		kind: InteractionResponseType::UpdateMessage,
		data: Some(response),
	};
	interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.await
		.into_diagnostic()?;

	Ok(())
}

async fn handle_staff_role_update(
	interaction: &InteractionCreate,
	interaction_data: &MessageComponentInteractionData,
	setup_id: &str,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let mut state = bot_state.write().await;
	let Some(setup_state) = state.get_mut::<SetupState>() else {
		return Ok(());
	};
	let interaction_client = http_client.interaction(application_id);
	let Some(interaction_state) = setup_state.states.get_mut(setup_id) else {
		let response = InteractionResponseDataBuilder::new()
			.content(SETUP_EXPIRED_MESSAGE)
			.components(Vec::new())
			.build();
		let response = InteractionResponse {
			kind: InteractionResponseType::UpdateMessage,
			data: Some(response),
		};
		interaction_client
			.create_response(interaction.id, &interaction.token, &response)
			.await
			.into_diagnostic()?;
		return Ok(());
	};

	let role = match interaction_data.values.first() {
		Some(role_str) => {
			let role_id: u64 = role_str.parse().into_diagnostic()?;
			let role: Id<RoleMarker> = Id::new(role_id);
			Some(role)
		}
		None => None,
	};
	interaction_state.staff_role = role;

	let updated_components = set_up_components(
		setup_id,
		interaction_state.admin_role.is_none() || interaction_state.staff_role.is_none(),
	);
	let response = InteractionResponseDataBuilder::new()
		.components(updated_components)
		.build();
	let response = InteractionResponse {
		kind: InteractionResponseType::UpdateMessage,
		data: Some(response),
	};
	interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.await
		.into_diagnostic()?;

	Ok(())
}

async fn handle_confirm(
	interaction: &InteractionCreate,
	setup_id: &str,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let mut state = bot_state.write().await;
	let Some(setup_state) = state.get_mut::<SetupState>() else {
		return Ok(());
	};
	let interaction_client = http_client.interaction(application_id);
	let Some(interaction_state) = setup_state.states.get_mut(setup_id) else {
		let response = InteractionResponseDataBuilder::new()
			.content(SETUP_EXPIRED_MESSAGE)
			.components(Vec::new())
			.build();
		let response = InteractionResponse {
			kind: InteractionResponseType::UpdateMessage,
			data: Some(response),
		};
		interaction_client
			.create_response(interaction.id, &interaction.token, &response)
			.await
			.into_diagnostic()?;
		return Ok(());
	};

	let (Some(admin_role), Some(staff_role)) = (interaction_state.admin_role, interaction_state.staff_role) else {
		let response = InteractionResponseDataBuilder::new()
			.content("Both roles must be selected to set up Twilight Sword.")
			.build();
		let response = InteractionResponse {
			kind: InteractionResponseType::ChannelMessageWithSource,
			data: Some(response),
		};
		interaction_client
			.create_response(interaction.id, &interaction.token, &response)
			.await
			.into_diagnostic()?;
		return Ok(());
	};

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;
	let guild_id = database_id_from_discord_id(interaction_state.guild.get());
	let admin_role = database_id_from_discord_id(admin_role.get());
	let staff_role = database_id_from_discord_id(staff_role.get());
	let guild_entry = Guild {
		guild_id,
		admin_role,
		staff_role,
		..Default::default()
	};
	let db_result = diesel::insert_into(guilds::table)
		.values(guild_entry)
		.execute(&mut db_connection);
	match db_result {
		Ok(_) => {
			let response = InteractionResponseDataBuilder::new()
				.content(
					"You've set up Twilight Sword! 🎉\nRemember to use `/settings` to configure other functionality.",
				)
				.components(Vec::new())
				.build();
			let response = InteractionResponse {
				kind: InteractionResponseType::UpdateMessage,
				data: Some(response),
			};
			interaction_client
				.create_response(interaction.id, &interaction.token, &response)
				.await
				.into_diagnostic()?;
		}
		Err(DbError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
			let response = InteractionResponseDataBuilder::new()
				.content("This server is already set up. Setup may have been completed elsewhere.")
				.components(Vec::new())
				.build();
			let response = InteractionResponse {
				kind: InteractionResponseType::UpdateMessage,
				data: Some(response),
			};
			interaction_client
				.create_response(interaction.id, &interaction.token, &response)
				.await
				.into_diagnostic()?;
		}
		Err(error) => {
			tracing::error!(source = ?error, "A database error occurred setting up a new guild");
			let response = InteractionResponseDataBuilder::new()
				.content("An internal error occurred setting up the server.")
				.components(Vec::new())
				.build();
			let response = InteractionResponse {
				kind: InteractionResponseType::UpdateMessage,
				data: Some(response),
			};
			interaction_client
				.create_response(interaction.id, &interaction.token, &response)
				.await
				.into_diagnostic()?;
		}
	};

	Ok(())
}

async fn handle_cancel(
	interaction: &InteractionCreate,
	setup_id: &str,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let mut state = bot_state.write().await;
	let setup_state = state.get_mut::<SetupState>();

	if let Some(state) = setup_state {
		state.states.remove(setup_id);
	}

	let interaction_client = http_client.interaction(application_id);
	let response = InteractionResponseDataBuilder::new()
		.content("Twilight Sword setup canceled.")
		.components(Vec::new())
		.build();
	let response = InteractionResponse {
		kind: InteractionResponseType::UpdateMessage,
		data: Some(response),
	};
	interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.await
		.into_diagnostic()?;

	Ok(())
}
