// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use kdl::KdlDocument;
use miette::{bail, miette, IntoDiagnostic};
use tokio::fs::read_to_string;

#[derive(Debug)]
pub struct ConfigData {
	pub discord_token: String,
	pub database: DatabaseArgs,
}

#[derive(Debug)]
pub struct DatabaseArgs {
	pub host: String,
	pub port: Option<u16>,
	pub username: String,
	pub password: String,
	pub database: String,
}

pub async fn parse_config(config_path: &str) -> miette::Result<ConfigData> {
	let config_file_contents = read_to_string(config_path).await.into_diagnostic()?;
	let config_document: KdlDocument = config_file_contents.parse()?;

	let Some(discord_token_node) = config_document.get("discord-token") else {
		bail!(miette!(code = "required::discord-token", "required discord-token"));
	};
	let Some(discord_token) = discord_token_node.get(0) else {
		bail!(
			miette!(code = "value::discord-token", "expected discord-token to have a value")
				.with_source_code(format!("{}", discord_token_node))
		);
	};
	let Some(discord_token) = discord_token.as_string() else {
		bail!(miette!(
			code = "type::discord-token",
			"expected discord-token value to be a string"
		)
		.with_source_code(format!("{}", discord_token_node)));
	};
	let discord_token = discord_token.to_string();

	let Some(database_args_node) = config_document.get("database") else {
		bail!(miette!(code = "required::database", "required database information"));
	};
	let Some(database_args) = database_args_node.children() else {
		bail!(
			miette!(code = "format::database", "expected databasse to have child nodes")
				.with_source_code(format!("{}", database_args_node))
		);
	};
	let Some(database_host) = database_args.get("host") else {
		bail!(miette!(
			code = "required::database::host",
			"required host property of database"
		));
	};
	let database_port = database_args.get("port");
	let Some(database_username) = database_args.get("username") else {
		bail!(miette!(
			code = "required::database::username",
			"required database::username"
		));
	};
	let Some(database_password) = database_args.get("password") else {
		bail!(miette!(
			code = "required::database::password",
			"required database::password"
		));
	};
	let Some(database_database) = database_args.get("database") else {
		bail!(miette!(
			code = "required::database::database",
			"required database::database"
		));
	};

	let Some(database_host) = database_host.get(0) else {
		bail!(
			miette!(code = "value::database::host", "expected database host to have a value")
				.with_source_code(format!("{}", database_args_node))
		);
	};
	let Some(database_host) = database_host.as_string() else {
		bail!(
			miette!(code = "type::database::host", "expected database host to be a string")
				.with_source_code(format!("{}", database_args_node))
		);
	};
	let database_host = database_host.to_string();

	let database_port = match database_port {
		Some(port_node) => {
			let Some(port) = port_node.get(0) else {
				bail!(
					miette!(code = "value::database::port", "expected database port to have a value")
						.with_source_code(format!("{}", database_args_node))
				);
			};
			let Some(port) = port.as_integer() else {
				bail!(miette!(
					code = "type::database::port",
					"expected database port to be an integer port number"
				)
				.with_source_code(format!("{}", database_args_node)));
			};
			let port: Option<u16> = match port.try_into() {
				Ok(port) => Some(port),
				Err(error) => bail!(miette!(
					code = "type::database::port",
					"expected database port number to be in range for port numbers ({})",
					error
				)
				.with_source_code(format!("{}", database_args_node))),
			};
			port
		}
		None => None,
	};

	let Some(database_username) = database_username.get(0) else {
		bail!(miette!(
			code = "value::database::username",
			"expected database username to have a value"
		)
		.with_source_code(format!("{}", database_args_node)));
	};
	let Some(database_username) = database_username.as_string() else {
		bail!(miette!(
			code = "type::database::username",
			"expected database username to be a string"
		)
		.with_source_code(format!("{}", database_args_node)));
	};
	let database_username = database_username.to_string();

	let Some(database_password) = database_password.get(0) else {
		bail!(miette!(
			code = "value::database::password",
			"expected database password to have a value"
		)
		.with_source_code(format!("{}", database_args_node)));
	};
	let Some(database_password) = database_password.as_string() else {
		bail!(miette!(
			code = "type::database::password",
			"expected database password to be a string"
		)
		.with_source_code(format!("{}", database_args_node)));
	};
	let database_password = database_password.to_string();

	let Some(database_database) = database_database.get(0) else {
		bail!(miette!(
			code = "value::database::database",
			"expected database database to have a value"
		)
		.with_source_code(format!("{}", database_args_node)));
	};
	let Some(database_database) = database_database.as_string() else {
		bail!(miette!(
			code = "type::database::database",
			"expected database name to be a string"
		)
		.with_source_code(format!("{}", database_args_node)));
	};
	let database_database = database_database.to_string();

	let database = DatabaseArgs {
		host: database_host,
		port: database_port,
		username: database_username,
		password: database_password,
		database: database_database,
	};

	let config = ConfigData {
		discord_token,
		database,
	};

	Ok(config)
}
