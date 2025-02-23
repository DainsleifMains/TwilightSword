// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use miette::Diagnostic;
use std::fmt;
use twilight_http::client::Client;
use twilight_http::error::Error;
use twilight_http::response::DeserializeBodyError;
use twilight_model::id::Id;
use twilight_model::id::marker::{GuildMarker, UserMarker};

/// User data obtained from guild member data, falling back to the user data if the guild member data is not available.
#[derive(Debug)]
pub struct UserData {
	pub display_name: String,
}

/// Error data for getting user/member data
#[derive(Debug, Diagnostic)]
pub enum UserDataError {
	Http(Error),
	Deserialize(DeserializeBodyError),
}

impl From<Error> for UserDataError {
	fn from(error: Error) -> Self {
		Self::Http(error)
	}
}

impl From<DeserializeBodyError> for UserDataError {
	fn from(error: DeserializeBodyError) -> Self {
		Self::Deserialize(error)
	}
}

impl std::error::Error for UserDataError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Http(error) => Some(error),
			Self::Deserialize(error) => Some(error),
		}
	}
}

impl fmt::Display for UserDataError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Http(error) => write!(f, "HTTP error: {}", error),
			Self::Deserialize(error) => write!(f, "deserialization error: {}", error),
		}
	}
}

/// Gets member data with a fallback to user data
pub async fn get_member_data(
	http_client: &Client,
	guild_id: Id<GuildMarker>,
	user_id: Id<UserMarker>,
) -> Result<UserData, UserDataError> {
	let member = get_member_data_only(http_client, guild_id, user_id).await;
	match member {
		Ok(member) => Ok(member),
		Err(_) => get_user_data_only(http_client, user_id).await,
	}
}

async fn get_member_data_only(
	http_client: &Client,
	guild_id: Id<GuildMarker>,
	user_id: Id<UserMarker>,
) -> Result<UserData, UserDataError> {
	let member_response = http_client.guild_member(guild_id, user_id).await?;
	let member = member_response.model().await?;

	let display_name = member.nick.or(member.user.global_name).unwrap_or(member.user.name);
	Ok(UserData { display_name })
}

async fn get_user_data_only(http_client: &Client, user_id: Id<UserMarker>) -> Result<UserData, UserDataError> {
	let user_response = http_client.user(user_id).await?;
	let user = user_response.model().await?;

	let display_name = user.global_name.unwrap_or(user.name);
	Ok(UserData { display_name })
}
