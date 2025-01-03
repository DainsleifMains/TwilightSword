// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::discord::interactions::MAX_INTERACTION_WAIT_TIME;
use crate::discord::state::setup::{set_up_components, SetupInstance, SetupState};
use crate::model::{database_id_from_discord_id, Guild as DbGuild};
use crate::schema::guilds;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::{bail, IntoDiagnostic};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::sleep;
use twilight_http::client::Client;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use twilight_util::builder::command::CommandBuilder;
use twilight_util::builder::InteractionResponseDataBuilder;
use type_map::concurrent::TypeMap;

pub fn command_definition() -> Command {
	CommandBuilder::new("setup", "Set up the bot for your guild", CommandType::ChatInput)
		.dm_permission(false)
		.default_member_permissions(Permissions::MANAGE_GUILD)
		.build()
}

pub async fn handle_command(
	interaction: &InteractionCreate,
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let Some(guild) = interaction.guild_id else {
		bail!("Setup command was used outside of a guild");
	};

	let mut db_connection = db_connection_pool.get().into_diagnostic()?;

	let db_guild: Option<DbGuild> = guilds::table
		.find(&database_id_from_discord_id(guild.get()))
		.first(&mut db_connection)
		.optional()
		.into_diagnostic()?;

	let interaction_client = http_client.interaction(application_id);

	if db_guild.is_some() {
		let message = InteractionResponseDataBuilder::new()
			.content("The server has already been set up! Use `/settings` to modify settings.")
			.flags(MessageFlags::EPHEMERAL)
			.build();
		let message = InteractionResponse {
			kind: InteractionResponseType::ChannelMessageWithSource,
			data: Some(message),
		};
		interaction_client
			.create_response(interaction.id, &interaction.token, &message)
			.await
			.into_diagnostic()?;
		return Ok(());
	}

	let setup_id = cuid2::create_id();

	let message_content = "In order to set up Twilight Sword, we only require a couple pieces of information (but they are required!).\nPlease specify the role given to administrators and the role given to all staff members. (You can change these later (for example, if you change your server's role setup).)\nThese settings are used to help determine who has permissions for various bot-related functionality.";
	let components = set_up_components(&setup_id, true);
	let message = InteractionResponseDataBuilder::new()
		.content(message_content)
		.components(components)
		.build();

	let interaction_client = http_client.interaction(application_id);
	let response = InteractionResponse {
		kind: InteractionResponseType::ChannelMessageWithSource,
		data: Some(message),
	};
	interaction_client
		.create_response(interaction.id, &interaction.token, &response)
		.await
		.into_diagnostic()?;

	{
		let mut state = bot_state.write().await;
		let set_up_state = state.entry::<SetupState>().or_insert_with(SetupState::default);
		let setup_instance = SetupInstance::new(guild, interaction.token.clone());
		set_up_state.states.insert(setup_id.clone(), setup_instance);
	}

	tokio::spawn(expire_setup(http_client, application_id, bot_state, setup_id));

	Ok(())
}

async fn expire_setup(
	http_client: Arc<Client>,
	application_id: Id<ApplicationMarker>,
	bot_state: Arc<RwLock<TypeMap>>,
	setup_id: String,
) {
	sleep(MAX_INTERACTION_WAIT_TIME).await;
	let mut state = bot_state.write().await;
	let Some(set_up_state) = state.get_mut::<SetupState>() else {
		return;
	};
	let Some(interaction_state) = set_up_state.states.remove(&setup_id) else {
		return;
	};

	let interaction_client = http_client.interaction(application_id);
	let _ = interaction_client
		.update_response(&interaction_state.initial_message_token)
		.content(Some(
			"Setup timed out. Run `/setup` again to set up Twilight Sword for your server!",
		))
		.components(None)
		.await;
}
