// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::web::pages::utils::{GuildParam, make_ticket_url};
use chrono::{DateTime, Utc};
use leptos::prelude::*;
use leptos_router::hooks::use_params;
use serde::{Deserialize, Serialize};

#[component]
pub fn OpenTickets() -> impl IntoView {
	let params = use_params::<GuildParam>();
	let guild_id = params.read().as_ref().ok().and_then(|params| params.guild);

	let active_tickets = OnceResource::new(get_active_tickets(guild_id));

	view! {
		<Transition fallback=|| view! { <div class="staff_ticket_list_loading">"Loading tickets..."</div> }>
			{
				move || match &active_tickets.read().as_ref().and_then(|tickets| tickets.as_ref().ok()) {
					Some(ticket_data) if !ticket_data.is_empty() => {
						view! {
							<table class="staff_ticket_list">
								<thead>
									<tr>
										<th>"Ticket"</th>
										<th>"User"</th>
										<th>"Last Message Author"</th>
										<th>"Last Message Time"</th>
									</tr>
								</thead>
								<tbody>
									{
										ticket_data.iter().map(|ticket|
											view! {
												<tr>
													<td>
														<a href={make_ticket_url(guild_id, &ticket.id)}>
															{ticket.title.clone()}
														</a>
													</td>
													<td>
														{ticket.with_user_name.clone()}
													</td>
													<td>
														{ticket.last_message_author_name.clone()}
													</td>
													<td>
														{ticket.last_message_time.to_rfc3339()}
													</td>
												</tr>
											}.into_any()
										).collect::<Vec<_>>()
									}
								</tbody>
							</table>
						}.into_any()
					}
					_ => view! {
						<div id="staff_ticket_list_empty">
							"No open tickets"
						</div>
					}.into_any()
				}
			}
		</Transition>
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StaffTicketMetadata {
	pub id: String,
	pub title: String,
	pub with_user_name: String,
	pub last_message_author_name: String,
	pub last_message_time: DateTime<Utc>,
}

#[server]
async fn get_active_tickets(guild_id: Option<u64>) -> Result<Vec<StaffTicketMetadata>, ServerFnError> {
	use crate::discord::utils::permissions::channel_permissions;
	use crate::model::{
		BuiltInTicketCategory, CustomCategory, Guild, Ticket, TicketMessage, database_id_from_discord_id,
	};
	use crate::schema::{custom_categories, guilds, ticket_messages, tickets};
	use crate::web::pages::server_utils::{get_guild_id_from_request, get_user_id_from_request};
	use crate::web::state::AppState;
	use diesel::prelude::*;
	use std::collections::HashMap;
	use std::collections::hash_map::Entry;
	use twilight_model::guild::Permissions;
	use twilight_model::id::Id;
	use twilight_model::id::marker::UserMarker;

	async fn to_ticket_metadata(
		ticket: Ticket,
		state: &AppState,
		db_connection: &mut PgConnection,
		usernames_cache: &mut HashMap<Id<UserMarker>, String>,
	) -> Result<StaffTicketMetadata, ServerFnError> {
		let last_message: TicketMessage = ticket_messages::table
			.filter(ticket_messages::ticket.eq(&ticket.id).and(ticket_messages::user_message.is_not_null()))
			.order(ticket_messages::send_time.desc())
			.first(db_connection)?;
		let with_user = ticket.get_with_user();
		let last_message_user = last_message.get_author();

		let with_user_name = match usernames_cache.entry(with_user) {
			Entry::Occupied(entry) => entry.get().clone(),
			Entry::Vacant(entry) => {
				let user = state.discord_client.user(with_user).await?.model().await?;
				entry.insert(user.name.clone());
				user.name
			}
		};

		let last_message_author_name = match usernames_cache.entry(last_message_user) {
			Entry::Occupied(entry) => entry.get().clone(),
			Entry::Vacant(entry) => {
				let user = state.discord_client.user(last_message_user).await?.model().await?;
				entry.insert(user.name.clone());
				user.name
			}
		};

		Ok(StaffTicketMetadata {
			id: ticket.id,
			title: ticket.title,
			with_user_name,
			last_message_author_name,
			last_message_time: last_message.send_time,
		})
	}

	let guild_id = get_guild_id_from_request(guild_id).await?;
	let user_id = get_user_id_from_request().await?;

	let (Some(guild_id), Some(user_id)) = (guild_id, user_id) else {
		return Err(ServerFnError::ServerError(String::from(
			"No guild found and/or user not logged in",
		)));
	};

	let state: AppState = expect_context();
	let mut db_connection = state.db_connection_pool.get()?;

	let db_guild_id = database_id_from_discord_id(guild_id.get());

	let guild: Guild = guilds::table.find(db_guild_id).first(&mut db_connection)?;

	let admin_role = guild.get_admin_role();
	let staff_role = guild.get_staff_role();

	let member = state
		.discord_client
		.guild_member(guild_id, user_id)
		.await?
		.model()
		.await?;

	if !member.roles.contains(&admin_role) && !member.roles.contains(&staff_role) {
		return Ok(Vec::new());
	}

	let all_tickets: Vec<Ticket> = tickets::table
		.filter(tickets::guild.eq(db_guild_id).and(tickets::closed_at.is_null()))
		.load(&mut db_connection)?;
	let mut tickets: Vec<StaffTicketMetadata> = Vec::with_capacity(all_tickets.len());

	let mut visible_for_category: HashMap<String, bool> = HashMap::new();
	let mut usernames: HashMap<Id<UserMarker>, String> = HashMap::new();

	for ticket in all_tickets {
		match (&ticket.built_in_category, &ticket.custom_category) {
			(Some(category), None) => {
				let category_id = match category {
					BuiltInTicketCategory::BanAppeal => "*0",
					BuiltInTicketCategory::NewPartner => "*1",
					BuiltInTicketCategory::ExistingPartner => "*2",
					BuiltInTicketCategory::MessageReport => "*3",
				};
				if let Some(&is_visible) = visible_for_category.get(category_id) {
					if is_visible {
						let ticket_metadata =
							to_ticket_metadata(ticket, &state, &mut db_connection, &mut usernames).await?;
						tickets.push(ticket_metadata);
					}
					continue;
				}

				let category_channel = match category {
					BuiltInTicketCategory::BanAppeal => guild.get_ban_appeal_ticket_channel(),
					BuiltInTicketCategory::NewPartner => guild.get_new_partner_ticket_channel(),
					BuiltInTicketCategory::ExistingPartner => guild.get_existing_partner_ticket_channel(),
					BuiltInTicketCategory::MessageReport => guild.get_message_reports_channel(),
				};
				let Some(category_channel) = category_channel else {
					visible_for_category.insert(category_id.to_string(), false);
					continue;
				};
				let permissions = channel_permissions(guild_id, category_channel, &state.discord_client).await;
				let permissions = match permissions {
					Ok(perms) => perms,
					Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
				};
				if !permissions.contains(Permissions::VIEW_CHANNEL) {
					visible_for_category.insert(category_id.to_string(), false);
					continue;
				}

				visible_for_category.insert(category_id.to_string(), true);

				let ticket_metadata = to_ticket_metadata(ticket, &state, &mut db_connection, &mut usernames).await?;
				tickets.push(ticket_metadata);
			}
			(None, Some(category)) => {
				if let Some(&is_visible) = visible_for_category.get(category) {
					if is_visible {
						let ticket_metadata =
							to_ticket_metadata(ticket, &state, &mut db_connection, &mut usernames).await?;
						tickets.push(ticket_metadata);
					}
					continue;
				}

				let custom_category: CustomCategory =
					custom_categories::table.find(category).first(&mut db_connection)?;
				let category_channel = custom_category.get_channel();
				let permissions = channel_permissions(guild_id, category_channel, &state.discord_client).await;
				let permissions = match permissions {
					Ok(perms) => perms,
					Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
				};
				if !permissions.contains(Permissions::VIEW_CHANNEL) {
					visible_for_category.insert(category.clone(), false);
					continue;
				}

				visible_for_category.insert(category.clone(), true);

				let ticket_metadata = to_ticket_metadata(ticket, &state, &mut db_connection, &mut usernames).await?;
				tickets.push(ticket_metadata);
			}
			_ => unreachable!(),
		}
	}

	Ok(tickets)
}
