// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::database::DatabaseConnection;
use crate::model::{database_id_from_discord_id, Guild as DbGuild};
use crate::schema::guilds;
use diesel::prelude::*;
use miette::{bail, IntoDiagnostic};
use serenity::builder::{
	CreateActionRow, CreateButton, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
	CreateSelectMenu, CreateSelectMenuKind, EditInteractionResponse,
};
use serenity::collector::ComponentInteractionCollector;
use serenity::model::application::{
	ButtonStyle, CommandInteraction, CommandType, ComponentInteraction, ComponentInteractionDataKind,
};
use serenity::model::id::RoleId;
use serenity::model::permissions::Permissions;
use serenity::prelude::*;
use std::time::Duration;

pub fn command_definition() -> CreateCommand {
	CreateCommand::new("setup")
		.kind(CommandType::ChatInput)
		.default_member_permissions(Permissions::MANAGE_GUILD)
		.dm_permission(false)
		.description("Set up the bot for your guild")
}

pub async fn execute(ctx: Context, command: CommandInteraction) -> miette::Result<()> {
	let Some(guild) = command.guild_id else {
		bail!("Setup command was used outside of a guild");
	};

	let mut db_connection = {
		let context_data = ctx.data.read().await;
		let db_connection_pool = context_data.get::<DatabaseConnection>().unwrap();
		db_connection_pool.get().into_diagnostic()?
	};

	let db_guild: Option<DbGuild> = guilds::table
		.find(&database_id_from_discord_id(guild.get()))
		.first(&mut db_connection)
		.optional()
		.into_diagnostic()?;

	if db_guild.is_some() {
		let message = CreateInteractionResponseMessage::new()
			.content("The server has already been set up! Use `/settings` to modify settings.")
			.ephemeral(true);
		command
			.create_response(&ctx.http, CreateInteractionResponse::Message(message))
			.await
			.into_diagnostic()?;
		return Ok(());
	}

	let admin_role_select_id = cuid2::create_id();
	let staff_role_select_id = cuid2::create_id();
	let set_up_button_id = cuid2::create_id();
	let cancel_button_id = cuid2::create_id();

	let admin_role_select = CreateSelectMenu::new(
		&admin_role_select_id,
		CreateSelectMenuKind::Role { default_roles: None },
	)
	.placeholder("Admin Role");
	let staff_role_select = CreateSelectMenu::new(
		&staff_role_select_id,
		CreateSelectMenuKind::Role { default_roles: None },
	)
	.placeholder("Staff Role");
	let set_up_button = CreateButton::new(&set_up_button_id)
		.label("Set Up!")
		.style(ButtonStyle::Primary)
		.disabled(true);
	let cancel_button = CreateButton::new(&cancel_button_id)
		.label("Cancel")
		.style(ButtonStyle::Secondary);

	let admin_role_row = CreateActionRow::SelectMenu(admin_role_select);
	let staff_role_row = CreateActionRow::SelectMenu(staff_role_select);
	let buttons_row = CreateActionRow::Buttons(vec![set_up_button.clone(), cancel_button.clone()]);

	let message = "In order to set up Twilight Sword, we only require a couple pieces of information (but they are required!).\nPlease specify the role given to administrators and the role given to all staff members. (You can change these later (for example, if you change your server's role setup).)";
	let message = CreateInteractionResponseMessage::new()
		.content(message)
		.components(vec![admin_role_row.clone(), staff_role_row.clone(), buttons_row]);

	command
		.create_response(&ctx.http, CreateInteractionResponse::Message(message))
		.await
		.into_diagnostic()?;

	let mut admin_role_id: Option<RoleId> = None;
	let mut staff_role_id: Option<RoleId> = None;

	let interaction: ComponentInteraction = loop {
		let interaction = ComponentInteractionCollector::new(&ctx.shard)
			.custom_ids(vec![
				admin_role_select_id.clone(),
				staff_role_select_id.clone(),
				set_up_button_id.clone(),
				cancel_button_id.clone(),
			])
			.timeout(Duration::from_secs(600))
			.await;
		let Some(interaction) = interaction else {
			let message = EditInteractionResponse::new()
				.content("Setup timed out. Run `/setup` again to set up Twilight Sword for your server!")
				.components(Vec::new());
			command.edit_response(&ctx.http, message).await.into_diagnostic()?;
			return Ok(());
		};
		match &interaction.data.kind {
			ComponentInteractionDataKind::RoleSelect { values } => {
				let value = values.first().copied();
				if interaction.data.custom_id == admin_role_select_id {
					admin_role_id = value;
				} else if interaction.data.custom_id == staff_role_select_id {
					staff_role_id = value;
				} else {
					continue;
				}

				let set_up_button = set_up_button
					.clone()
					.disabled(admin_role_id.is_none() || staff_role_id.is_none());
				let buttons_row = CreateActionRow::Buttons(vec![set_up_button, cancel_button.clone()]);
				let message = EditInteractionResponse::new().components(vec![
					admin_role_row.clone(),
					staff_role_row.clone(),
					buttons_row,
				]);
				command.edit_response(&ctx.http, message).await.into_diagnostic()?;
				interaction
					.create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
					.await
					.into_diagnostic()?;
			}
			ComponentInteractionDataKind::Button => {
				if interaction.data.custom_id == set_up_button_id {
					let set_up_button = set_up_button.clone().disabled(true);
					let cancel_button = cancel_button.clone().disabled(true);
					let buttons_row = CreateActionRow::Buttons(vec![set_up_button, cancel_button]);
					let edit_response =
						EditInteractionResponse::new().components(vec![admin_role_row, staff_role_row, buttons_row]);
					command
						.edit_response(&ctx.http, edit_response)
						.await
						.into_diagnostic()?;
					break interaction;
				} else if interaction.data.custom_id == cancel_button_id {
					let message = EditInteractionResponse::new()
						.content("Twilight Sword setup canceled.")
						.components(Vec::new());
					command.edit_response(&ctx.http, message).await.into_diagnostic()?;
					interaction
						.create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
						.await
						.into_diagnostic()?;
					return Ok(());
				}
			}
			_ => bail!(
				"Unexpected interaction type for setup command: {:?}",
				interaction.data.kind
			),
		}
	};

	let Some(admin_role_id) = admin_role_id else {
		bail!("Setup continued with no admin role selected");
	};
	let Some(staff_role_id) = staff_role_id else {
		bail!("Setup continued with no staff role selected");
	};

	let guild_data = DbGuild {
		guild_id: database_id_from_discord_id(guild.get()),
		admin_role: database_id_from_discord_id(admin_role_id.get()),
		staff_role: database_id_from_discord_id(staff_role_id.get()),
		..Default::default()
	};
	let db_result = diesel::insert_into(guilds::table)
		.values(guild_data)
		.execute(&mut db_connection);

	match db_result {
		Ok(_) => {
			let message = CreateInteractionResponseMessage::new().content(
				"You've set up Twilight Sword! 🎉\nRemember to use `/settings` to configure other functionality.",
			);
			interaction
				.create_response(&ctx.http, CreateInteractionResponse::Message(message))
				.await
				.into_diagnostic()?;
		}
		Err(db_error) => {
			eprintln!("A setup error occurred: {:?}", db_error);
			let message =
				CreateInteractionResponseMessage::new().content("An internal error occurred completing setup.");
			interaction
				.create_response(&ctx.http, CreateInteractionResponse::Message(message))
				.await
				.into_diagnostic()?;
		}
	}

	Ok(())
}
