// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serenity::builder::CreateCommand;
use serenity::model::application::CommandInteraction;
use serenity::prelude::*;

mod setup;

pub fn command_definitions() -> Vec<CreateCommand> {
	vec![setup::command_definition()]
}

pub async fn route_command(ctx: Context, command: CommandInteraction) -> miette::Result<()> {
	match command.data.name.as_str() {
		"setup" => setup::execute(ctx, command).await,
		_ => unimplemented!(),
	}
}
