// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::web::pages::utils::{GuildParam, make_ticket_url};
use crate::web::permissions::PermissionLevel;
use chrono::{DateTime, Utc};
use leptos::prelude::*;
use leptos_router::hooks::use_params;
use serde::{Deserialize, Serialize};

#[component]
pub fn Dashboard() -> impl IntoView {
	let params = use_params::<GuildParam>();
	let guild_id = params.read().as_ref().ok().and_then(|params| params.guild);

	let permission_level = Resource::new(|| (), move |_| fetch_permission_level(guild_id));
	let permission_level = move || {
		permission_level
			.read()
			.as_ref()
			.map(|level| level.clone().unwrap_or_default())
			.unwrap_or_default()
	};

	let user_active_tickets = OnceResource::new(get_active_tickets_for_user(guild_id));
	let user_closed_tickets = OnceResource::new(get_closed_tickets_for_user(guild_id));

	view! {
		<div id="dashboard_layout">
			<div id="dashboard_tickets">
				<Transition fallback=|| view! { <div class="dashboard_ticket_list_loading">"Loading tickets..."</div> }>
					<table class="dashboard_ticket_list">
						<thead>
							<tr>
								<th>"Ticket"</th>
								<th>"Last Message Author"</th>
								<th>"Last Message Time"</th>
							</tr>
						</thead>
						<tbody>
							{
								move || match &user_active_tickets.read().as_ref().and_then(|tickets| tickets.as_ref().ok()) {
									Some(ticket_data) if !ticket_data.is_empty() => {
										ticket_data.iter().map(|ticket|
											view! {
												<tr>
													<td>
														<a href={make_ticket_url(guild_id, &ticket.id)}>
															{ticket.title.clone()}
														</a>
													</td>
													<td class="dashboard_ticket_list_author">
														{ticket.last_message_author_name.clone()}
													</td>
													<td>
														{ticket.last_message_time.to_rfc3339()}
													</td>
												</tr>
											}.into_any()
										).collect::<Vec<_>>()
									}
									_ => {
										let no_tickets_view = view! {
											<tr>
												<td colspan={3} class="dashboard_ticket_list_no_tickets">
													"No open tickets"
												</td>
											</tr>
										}.into_any();
										vec![no_tickets_view]
									}
								}
							}
						</tbody>
					</table>
				</Transition>
				<Transition fallback=|| view! { <div class="dashboard_ticket_list_loading">"Loading tickets..."</div> }>
					{
						move || match &user_closed_tickets.read().as_ref().and_then(|tickets| tickets.as_ref().ok()) {
							Some(ticket_data) if !ticket_data.is_empty() => {
								Some(view! {
									<table class="dashboard_ticket_list">
										<thead>
											<tr>
												<th>"Ticket"</th>
												<th>"Closed"</th>
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
																{ticket.closed_at.to_rfc3339()}
															</td>
														</tr>
													}
												).collect::<Vec<_>>()
											}
										</tbody>
									</table>
								})
							}
							_ => None
						}
					}
				</Transition>
			</div>
			<Transition>
				<Show when=move || permission_level() != PermissionLevel::Member>
					<div id="dashboard_staff">
						<h2>Staff Menu</h2>
						<ul>
							<li>
								<a href={make_staff_open_ticket_list_url(guild_id)}>
									"Open Tickets"
								</a>
							</li>
							<li>
								<a href={make_form_manager_url(guild_id)}>
									"Form Manager"
								</a>
							</li>
						</ul>

						<Show when=move || permission_level() == PermissionLevel::Admin>
							<h2>Admin Menu</h2>
						</Show>
					</div>
				</Show>
			</Transition>
		</div>
	}
}

#[server]
pub async fn fetch_permission_level(guild_id: Option<u64>) -> Result<PermissionLevel, ServerFnError> {
	use crate::model::{Guild, database_id_from_discord_id};
	use crate::schema::guilds;
	use crate::web::pages::server_utils::{get_guild_id_from_request, get_user_id_from_request};
	use crate::web::state::AppState;
	use diesel::prelude::*;

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

	if member.roles.contains(&admin_role) {
		Ok(PermissionLevel::Admin)
	} else if member.roles.contains(&staff_role) {
		Ok(PermissionLevel::Staff)
	} else {
		Ok(PermissionLevel::Member)
	}
}

/// Makes a URL to the list of open tickets for staff
fn make_staff_open_ticket_list_url(guild_id: Option<u64>) -> String {
	match guild_id {
		Some(id) => format!("/{}/staff/open_tickets", id),
		None => String::from("/staff/open_tickets"),
	}
}

/// Makes a URL to the form manager
fn make_form_manager_url(guild_id: Option<u64>) -> String {
	match guild_id {
		Some(id) => format!("/{}/staff/manage_forms", id),
		None => String::from("/staff/manage_forms")
	}
}

/// Information about active tickets for showing on the dashboard
#[derive(Debug, Deserialize, Serialize)]
pub struct ActiveTicketMetadata {
	id: String,
	title: String,
	last_message_author_name: String,
	last_message_time: DateTime<Utc>,
}

/// Information about closed tickets for showing on the dashboard
#[derive(Debug, Deserialize, Serialize)]
pub struct ClosedTicketMetadata {
	id: String,
	title: String,
	closed_at: DateTime<Utc>,
}

/// Gets all active tickets for the current user.
///
/// Requires the guild ID parameter from the URL for correct guild lookup.
#[server]
async fn get_active_tickets_for_user(guild_id: Option<u64>) -> Result<Vec<ActiveTicketMetadata>, ServerFnError> {
	use crate::discord::utils::users::get_member_data;
	use crate::model::{Ticket, TicketMessage, database_id_from_discord_id};
	use crate::schema::{ticket_messages, tickets};
	use crate::web::pages::server_utils::{get_guild_id_from_request, get_user_id_from_request};
	use crate::web::state::AppState;
	use diesel::prelude::*;

	let guild_id = get_guild_id_from_request(guild_id).await?;
	let user_id = get_user_id_from_request().await?;

	let (Some(guild_id), Some(user_id)) = (guild_id, user_id) else {
		return Ok(Vec::new());
	};

	let db_guild_id = database_id_from_discord_id(guild_id.get());
	let db_user_id = database_id_from_discord_id(user_id.get());

	let state: AppState = expect_context();
	let mut db_connection = state.db_connection_pool.get()?;

	let user_tickets: Vec<Ticket> = tickets::table
		.filter(
			tickets::guild
				.eq(db_guild_id)
				.and(tickets::with_user.eq(db_user_id))
				.and(tickets::closed_at.is_null()),
		)
		.load(&mut db_connection)?;

	let mut tickets: Vec<ActiveTicketMetadata> = Vec::with_capacity(user_tickets.len());
	for ticket in user_tickets {
		let last_message: TicketMessage = ticket_messages::table
			.filter(
				ticket_messages::ticket
					.eq(&ticket.id)
					.and(ticket_messages::user_message.is_not_null()),
			)
			.order(ticket_messages::send_time.desc())
			.first(&mut db_connection)?;
		let author_id = last_message.get_author();

		let author_data = get_member_data(&state.discord_client, guild_id, author_id).await;
		let author_name = match author_data {
			Ok(data) => data.display_name,
			Err(_) => format!("<{}>", author_id.get()),
		};

		let ticket_metadata = ActiveTicketMetadata {
			id: ticket.id,
			title: ticket.title,
			last_message_author_name: author_name,
			last_message_time: last_message.send_time,
		};
		tickets.push(ticket_metadata);
	}
	Ok(tickets)
}

/// Gets closed tickets for a user.
///
/// Requires the guild ID parameter from the URL for accurate guild lookup.
#[server]
async fn get_closed_tickets_for_user(guild_id: Option<u64>) -> Result<Vec<ClosedTicketMetadata>, ServerFnError> {
	use crate::model::{Ticket, database_id_from_discord_id};
	use crate::schema::tickets;
	use crate::web::pages::server_utils::{get_guild_id_from_request, get_user_id_from_request};
	use crate::web::state::AppState;
	use diesel::prelude::*;

	let guild_id = get_guild_id_from_request(guild_id).await?;
	let user_id = get_user_id_from_request().await?;

	let (Some(guild_id), Some(user_id)) = (guild_id, user_id) else {
		return Ok(Vec::new());
	};

	let db_guild_id = database_id_from_discord_id(guild_id.get());
	let db_user_id = database_id_from_discord_id(user_id.get());

	let state: AppState = expect_context();
	let mut db_connection = state.db_connection_pool.get()?;

	let user_tickets: Vec<Ticket> = tickets::table
		.filter(
			tickets::guild
				.eq(db_guild_id)
				.and(tickets::with_user.eq(db_user_id))
				.and(tickets::closed_at.is_not_null()),
		)
		.order(tickets::closed_at.desc())
		.load(&mut db_connection)?;

	let tickets: Vec<ClosedTicketMetadata> = user_tickets
		.into_iter()
		.map(|ticket| ClosedTicketMetadata {
			id: ticket.id,
			title: ticket.title,
			closed_at: ticket.closed_at.unwrap(),
		})
		.collect();

	Ok(tickets)
}
