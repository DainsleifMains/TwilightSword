// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::commands::{command_definitions, route_command};
use serenity::async_trait;
use serenity::model::application::{Command, Interaction};
use serenity::model::gateway::Ready;
use serenity::prelude::*;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
	async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
		if let Interaction::Command(command) = interaction {
			let command_result = route_command(ctx, command).await;
			if let Err(error) = command_result {
				eprintln!("Command error: {}", error);
			}
		}
	}

	async fn ready(&self, ctx: Context, _data_about_bot: Ready) {
		let commands = command_definitions();
		Command::set_global_commands(&ctx.http, commands)
			.await
			.expect("Commands should have registered successfully");
	}
}
