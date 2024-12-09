// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod config;

#[tokio::main]
async fn main() -> miette::Result<()> {
	let config = config::parse_config("config.kdl").await?;

	println!("{:?}", config);

	Ok(())
}
