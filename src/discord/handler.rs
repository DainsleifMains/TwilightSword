// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serenity::async_trait;
use serenity::builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::{Command, CommandType, Interaction};
use serenity::model::gateway::Ready;
use serenity::prelude::*;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
	async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
		if let Interaction::Command(command) = interaction {
			if command.data.name.as_str() == "ping" {
				let message = CreateInteractionResponseMessage::new().content("Pong!");
				let _ = command
					.create_response(&ctx.http, CreateInteractionResponse::Message(message))
					.await;
			}
		}
	}

	async fn ready(&self, ctx: Context, _data_about_bot: Ready) {
		let ping_command = CreateCommand::new("ping")
			.kind(CommandType::ChatInput)
			.description("Pings");
		let commands = vec![ping_command];
		Command::set_global_commands(&ctx.http, commands)
			.await
			.expect("Commands registered");
	}
}
