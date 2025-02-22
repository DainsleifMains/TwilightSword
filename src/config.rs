// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use kdl::KdlDocument;
use miette::{IntoDiagnostic, bail, miette};
use tokio::fs::read_to_string;

#[derive(Debug)]
pub struct ConfigData {
	pub discord: DiscordArgs,
	pub database: DatabaseArgs,
	pub web: WebArgs,
}

#[derive(Debug)]
pub struct DiscordArgs {
	pub bot_token: String,
	pub client_id: String,
	pub client_secret: String,
}

#[derive(Debug)]
pub struct DatabaseArgs {
	pub host: String,
	pub port: Option<u16>,
	pub username: String,
	pub password: String,
	pub database: String,
}

#[derive(Debug)]
pub struct WebArgs {
	pub bind_addr: String,
	pub base_url: String,
}

pub async fn parse_config(config_path: &str) -> miette::Result<ConfigData> {
	let config_file_contents = read_to_string(config_path).await.into_diagnostic()?;
	let config_document: KdlDocument = config_file_contents.parse()?;

	let Some(discord_args_node) = config_document.get("discord") else {
		bail!(miette!(code = "required::discord", "required discord information"));
	};
	let Some(discord_args) = discord_args_node.children() else {
		bail!(miette!(
			code = "format::discord",
			"expected discord to have child nodes"
		));
	};
	let Some(discord_bot_token) = discord_args.get("bot-token") else {
		bail!(miette!(
			code = "required::discord::bot-token",
			"required bot-token property of discord"
		));
	};
	let Some(discord_client_id) = discord_args.get("client-id") else {
		bail!(miette!(
			code = "required::discord::client-id",
			"required client-id property of discord"
		));
	};
	let Some(discord_client_secret) = discord_args.get("client-secret") else {
		bail!(miette!(
			code = "required::discord::client-secret",
			"required client-secret property of discord"
		));
	};

	let Some(discord_bot_token) = discord_bot_token.get(0) else {
		bail!(
			miette!(
				code = "value::discord::bot-token",
				"expected discord bot token to have a value"
			)
			.with_source_code(format!("{}", discord_args_node))
		);
	};
	let Some(discord_bot_token) = discord_bot_token.as_string() else {
		bail!(
			miette!(
				code = "type::discord::bot-token",
				"expected discord bot token to be a string"
			)
			.with_source_code(format!("{}", discord_args_node))
		);
	};
	let discord_bot_token = discord_bot_token.to_string();

	let Some(discord_client_id) = discord_client_id.get(0) else {
		bail!(
			miette!(
				code = "value::discord::client-id",
				"expected discord client ID to have a value"
			)
			.with_source_code(format!("{}", discord_args_node))
		);
	};
	let Some(discord_client_id) = discord_client_id.as_string() else {
		bail!(
			miette!(
				code = "type::discord::client-id",
				"expected discord client ID to be a string"
			)
			.with_source_code(format!("{}", discord_args_node))
		);
	};
	let discord_client_id = discord_client_id.to_string();

	let Some(discord_client_secret) = discord_client_secret.get(0) else {
		bail!(
			miette!(
				code = "value::discord::client-secret",
				"expected discord client secret to have a value"
			)
			.with_source_code(format!("{}", discord_args_node))
		);
	};
	let Some(discord_client_secret) = discord_client_secret.as_string() else {
		bail!(
			miette!(
				code = "type::discord::client-secret",
				"expected discord client secret to be a string"
			)
			.with_source_code(format!("{}", discord_args_node))
		);
	};
	let discord_client_secret = discord_client_secret.to_string();

	let discord = DiscordArgs {
		bot_token: discord_bot_token,
		client_id: discord_client_id,
		client_secret: discord_client_secret,
	};

	let Some(database_args_node) = config_document.get("database") else {
		bail!(miette!(code = "required::database", "required database information"));
	};
	let Some(database_args) = database_args_node.children() else {
		bail!(
			miette!(code = "format::database", "expected database to have child nodes")
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
				bail!(
					miette!(
						code = "type::database::port",
						"expected database port to be an integer port number"
					)
					.with_source_code(format!("{}", database_args_node))
				);
			};
			let port: Option<u16> = match port.try_into() {
				Ok(port) => Some(port),
				Err(error) => bail!(
					miette!(
						code = "type::database::port",
						"expected database port number to be in range for port numbers ({})",
						error
					)
					.with_source_code(format!("{}", database_args_node))
				),
			};
			port
		}
		None => None,
	};

	let Some(database_username) = database_username.get(0) else {
		bail!(
			miette!(
				code = "value::database::username",
				"expected database username to have a value"
			)
			.with_source_code(format!("{}", database_args_node))
		);
	};
	let Some(database_username) = database_username.as_string() else {
		bail!(
			miette!(
				code = "type::database::username",
				"expected database username to be a string"
			)
			.with_source_code(format!("{}", database_args_node))
		);
	};
	let database_username = database_username.to_string();

	let Some(database_password) = database_password.get(0) else {
		bail!(
			miette!(
				code = "value::database::password",
				"expected database password to have a value"
			)
			.with_source_code(format!("{}", database_args_node))
		);
	};
	let Some(database_password) = database_password.as_string() else {
		bail!(
			miette!(
				code = "type::database::password",
				"expected database password to be a string"
			)
			.with_source_code(format!("{}", database_args_node))
		);
	};
	let database_password = database_password.to_string();

	let Some(database_database) = database_database.get(0) else {
		bail!(
			miette!(
				code = "value::database::database",
				"expected database database to have a value"
			)
			.with_source_code(format!("{}", database_args_node))
		);
	};
	let Some(database_database) = database_database.as_string() else {
		bail!(
			miette!(
				code = "type::database::database",
				"expected database name to be a string"
			)
			.with_source_code(format!("{}", database_args_node))
		);
	};
	let database_database = database_database.to_string();

	let database = DatabaseArgs {
		host: database_host,
		port: database_port,
		username: database_username,
		password: database_password,
		database: database_database,
	};

	let Some(web_args_node) = config_document.get("web") else {
		bail!(miette!(code = "required::web", "required web"));
	};
	let Some(web_args) = web_args_node.children() else {
		bail!(miette!(code = "format::web", "expected web to have child nodes"));
	};
	let Some(web_bind_addr) = web_args.get("bind-addr") else {
		bail!(miette!(
			code = "required::web::bind-addr",
			"required bind-addr property of web"
		));
	};
	let Some(web_base_url) = web_args.get("base-url") else {
		bail!(miette!(
			code = "required::web::base-url",
			"required base-url property of web"
		));
	};

	let Some(web_bind_addr) = web_bind_addr.get(0) else {
		bail!(miette!(
			code = "value::web::bind-addr",
			"expected web bind-addr to have a value"
		));
	};
	let Some(web_bind_addr) = web_bind_addr.as_string() else {
		bail!(miette!(
			code = "type::web::bind-addr",
			"expected web bind-addr to be a string"
		));
	};
	let web_bind_addr = web_bind_addr.to_string();

	let Some(web_base_url) = web_base_url.get(0) else {
		bail!(miette!(
			code = "value::web::base-url",
			"expected web base-url to have a value"
		));
	};
	let Some(web_base_url) = web_base_url.as_string() else {
		bail!(miette!(
			code = "type::web::base-url",
			"expected web base-url to be a string"
		));
	};
	let web_base_url = web_base_url.to_string();

	let web = WebArgs {
		bind_addr: web_bind_addr,
		base_url: web_base_url,
	};

	let config = ConfigData { discord, database, web };

	Ok(config)
}
