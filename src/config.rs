// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use knus::Decode;
use miette::{IntoDiagnostic, Result};
use tokio::fs::read_to_string;

pub async fn parse_config(config_path: &str) -> Result<ConfigDocument> {
	let config_file_contents = read_to_string(config_path).await.into_diagnostic()?;
	let config = knus::parse(config_path, &config_file_contents)?;
	Ok(config)
}

#[derive(Debug, Decode)]
pub struct ConfigDocument {
	#[knus(child, unwrap(argument))]
	pub discord_token: String,
}
