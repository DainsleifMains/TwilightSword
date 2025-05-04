// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
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
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use twilight_util::builder::command::CommandBuilder;
use type_map::concurrent::TypeMap;

mod action_reason_complain_channel;
mod admin_role;
mod ban_appeal_ticket;
mod custom_categories;
mod existing_partner_ticket;
mod message_reports_channel;
mod new_partner_ticket;
mod staff_role;
mod start_ticket_channel;
mod start_ticket_message;

pub fn command_definition() -> Command {
	CommandBuilder::new(
		"settings",
		"View or modify settings for your server",
		CommandType::ChatInput,
	)
	.contexts([InteractionContextType::Guild])
	.default_member_permissions(Permissions::MANAGE_GUILD)
	.option(action_reason_complain_channel::subcommand_definition())
	.option(admin_role::subcommand_definition())
	.option(ban_appeal_ticket::subcommand_definition())
	.option(custom_categories::subcommand_definition())
	.option(existing_partner_ticket::subcommand_definition())
	.option(message_reports_channel::subcommand_definition())
	.option(new_partner_ticket::subcommand_definition())
	.option(staff_role::subcommand_definition())
	.option(start_ticket_channel::subcommand_definition())
	.option(start_ticket_message::subcommand_definition())
	.build()
}

pub async fn handle_command(
	interaction: &InteractionCreate,
	command_data: &CommandData,
	http_client: &Client,
	application_id: Id<ApplicationMarker>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	bot_state: Arc<RwLock<TypeMap>>,
) -> miette::Result<()> {
	let Some(subcommand_data) = command_data.options.first() else {
		bail!("Settings command invoked with no subcommand");
	};

	match subcommand_data.name.as_str() {
		"action_reason_complain_channel" => {
			action_reason_complain_channel::handle_subcommand(
				interaction,
				&subcommand_data.value,
				http_client,
				application_id,
				db_connection_pool,
			)
			.await
		}
		"admin_role" => {
			admin_role::handle_subcommand(
				interaction,
				&subcommand_data.value,
				http_client,
				application_id,
				db_connection_pool,
			)
			.await
		}
		"ban_appeal_ticket" => {
			ban_appeal_ticket::handle_subcommand(
				interaction,
				&subcommand_data.value,
				http_client,
				application_id,
				db_connection_pool,
				bot_state,
			)
			.await
		}
		"custom_categories" => {
			custom_categories::handle_subcommand(
				interaction,
				&subcommand_data.value,
				http_client,
				application_id,
				db_connection_pool,
			)
			.await
		}
		"existing_partner_ticket" => {
			existing_partner_ticket::handle_subcommand(
				interaction,
				&subcommand_data.value,
				http_client,
				application_id,
				db_connection_pool,
			)
			.await
		}
		"message_reports_channel" => {
			message_reports_channel::handle_subcommand(
				interaction,
				&subcommand_data.value,
				http_client,
				application_id,
				db_connection_pool,
			)
			.await
		}
		"new_partner_ticket" => {
			new_partner_ticket::handle_subcommand(
				interaction,
				&subcommand_data.value,
				http_client,
				application_id,
				db_connection_pool,
			)
			.await
		}
		"staff_role" => {
			staff_role::handle_subcommand(
				interaction,
				&subcommand_data.value,
				http_client,
				application_id,
				db_connection_pool,
			)
			.await
		}
		"start_ticket_channel" => {
			start_ticket_channel::handle_subcommand(
				interaction,
				&subcommand_data.value,
				http_client,
				application_id,
				db_connection_pool,
			)
			.await
		}
		"start_ticket_message" => {
			start_ticket_message::handle_subcommand(
				interaction,
				http_client,
				application_id,
				db_connection_pool,
				bot_state,
			)
			.await
		}
		_ => bail!(
			"Unknown settings subcommand encountered: {}\n{:?}",
			subcommand_data.name,
			subcommand_data
		),
	}
}
