// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use miette::IntoDiagnostic;
use std::collections::HashMap;
use std::future::IntoFuture;
use twilight_http::client::Client;
use twilight_http::error::ErrorType;
use twilight_http::response::StatusCode;
use twilight_model::guild::Permissions;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, RoleMarker};
use twilight_model::id::Id;
use twilight_util::permission_calculator::PermissionCalculator;

pub async fn channel_permissions(
	guild_id: Id<GuildMarker>,
	channel_id: Id<ChannelMarker>,
	http_client: &Client,
) -> miette::Result<Permissions> {
	let self_user = http_client
		.current_user()
		.await
		.into_diagnostic()?
		.model()
		.await
		.into_diagnostic()?;

	let self_member_future = http_client.guild_member(guild_id, self_user.id).into_future();
	let channel_data_future = http_client.channel(channel_id).into_future();
	let guild_roles_future = http_client.roles(guild_id).into_future();
	let (self_member, channel_data, guild_roles) =
		tokio::join!(self_member_future, channel_data_future, guild_roles_future);

	let self_member = self_member.into_diagnostic()?.model().await.into_diagnostic()?;
	let guild_roles = guild_roles.into_diagnostic()?.models().await.into_diagnostic()?;

	let channel_data = match channel_data {
		Ok(response) => response.model().await.into_diagnostic()?,
		Err(error) => {
			if let ErrorType::Response { status, .. } = error.kind() {
				if *status == StatusCode::FORBIDDEN {
					return Ok(Permissions::empty());
				}
			}
			return Err(error).into_diagnostic();
		}
	};

	let guild_everyone_role_id: Id<RoleMarker> = guild_id.cast();
	let role_permissions: HashMap<Id<RoleMarker>, Permissions> =
		guild_roles.iter().map(|role| (role.id, role.permissions)).collect();
	let everyone_role_permissions = role_permissions
		.get(&guild_everyone_role_id)
		.copied()
		.unwrap_or_else(Permissions::empty);
	let member_roles: Vec<(Id<RoleMarker>, Permissions)> = self_member
		.roles
		.iter()
		.map(|role_id| {
			(
				*role_id,
				role_permissions
					.get(role_id)
					.copied()
					.unwrap_or_else(Permissions::empty),
			)
		})
		.collect();
	let channel_permission_overwrites = channel_data.permission_overwrites.unwrap_or_default();

	let calculator = PermissionCalculator::new(guild_id, self_user.id, everyone_role_permissions, &member_roles);
	Ok(calculator.in_channel(channel_data.kind, &channel_permission_overwrites))
}
