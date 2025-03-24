// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::not_found::NotFound;
use crate::web::pages::utils::{TicketData, TicketParams};
use leptos::prelude::*;
use leptos_router::hooks::use_params;

#[component]
pub fn TicketPage() -> impl IntoView {
	let params = use_params::<TicketParams>();
	let params = params.read();
	let params = params.as_ref().ok().cloned();

	let Some(params) = params else {
		return view! { <NotFound /> }.into_any();
	};

	let Some(ticket_id) = params.ticket.clone() else {
		return view! { <NotFound /> }.into_any();
	};

	let ticket = Resource::new(|| (), move |_| get_ticket_data(params.guild, ticket_id.clone()));

	view! {
		<Transition fallback=|| view! { <div id="ticket_view_loading">"Loading ticket..."</div> }>
			{
				move || match &ticket.read().as_ref().and_then(|ticket| ticket.as_ref().ok()).flatten() {
					Some(ticket) => {
						let (ticket_title, _) = signal(ticket.title.clone());
						let (ticket_category, _) = signal(ticket.category_name.clone());
						let (ticket_messages, _) = signal(ticket.messages.clone());
						view! {
							<div id="ticket_header">
								<h1 id="ticket_title">{ticket_title.get()}</h1>
								<div id="ticket_category">{ticket_category.get()}</div>
							</div>
							<div id="ticket_message_list">
								<For
									each=move || ticket_messages.get()
									key=|message| message.id.clone()
									children=|message| {
										view! {
											<div class="ticket_message">
												<div class="ticket_message_start">
													<span class="ticket_message_author">
														{message.author_name}
													</span>
													<span class="ticket_message_time">
														{message.send_time.to_rfc3339()}
													</span>
													{
														move || if message.internal {
															view! {
																<span class="ticket_message_internal">
																	"Internal"
																</span>
															}.into_any()
														} else {
															().into_any()
														}
													}
												</div>
												<div class="ticket_message_body">
													{message.body}
												</div>
											</div>
										}
									}
								/>
							</div>
						}.into_any()
					}
					None => {
						view! { <NotFound /> }.into_any()
					}
				}
			}
		</Transition>
	}
	.into_any()
}

#[server]
async fn get_ticket_data(client_guild_id: Option<u64>, ticket_id: String) -> Result<Option<TicketData>, ServerFnError> {
	use crate::discord::utils::permissions::channel_permissions;
	use crate::model::{
		BuiltInTicketCategory, CustomCategory, Guild, Ticket, TicketMessage as TicketMessageDb,
		database_id_from_discord_id,
	};
	use crate::schema::{custom_categories, guilds, ticket_messages, tickets};
	use crate::web::pages::server_utils::{get_guild_data_from_request, get_user_id_from_request};
	use crate::web::pages::utils::TicketMessage as TicketMessageWeb;
	use crate::web::state::AppState;
	use diesel::prelude::*;
	use std::collections::{HashMap, HashSet};
	use std::sync::Arc;
	use tokio::task::JoinSet;
	use twilight_model::guild::Permissions;
	use twilight_model::id::Id;
	use twilight_model::id::marker::UserMarker;

	let guild_data = get_guild_data_from_request(client_guild_id).await?;
	let Some(guild_data) = guild_data else {
		return Ok(None);
	};
	let request_user = get_user_id_from_request().await?;
	let Some(request_user) = request_user else {
		return Ok(None);
	};

	let guild_id = guild_data.get_guild_id();

	let state: AppState = expect_context();
	let mut db_connection = state.db_connection_pool.get()?;

	let ticket: Option<Ticket> = tickets::table
		.filter(tickets::id.eq(&ticket_id).and(tickets::guild.eq(guild_data.guild_id)))
		.first(&mut db_connection)
		.optional()?;
	let Some(ticket) = ticket else {
		return Ok(None);
	};

	let ticket_title = ticket.title.clone();
	let category_name = match (&ticket.built_in_category, &ticket.custom_category) {
		(Some(category), None) => format!("{}", category),
		(None, Some(category_id)) => {
			let category: CustomCategory = custom_categories::table.find(category_id).first(&mut db_connection)?;
			category.name
		}
		_ => return Ok(None),
	};

	let ticket_with_user = ticket.get_with_user();

	let discord_client = state.discord_client.clone();

	let ticket_messages_db: Vec<TicketMessageDb> = if request_user == ticket_with_user {
		ticket_messages::table
			.filter(
				ticket_messages::ticket
					.eq(&ticket_id)
					.and(ticket_messages::user_message.is_not_null()),
			)
			.load(&mut db_connection)?
	} else {
		let user_response = discord_client.guild_member(guild_id, request_user).await?;
		let user = user_response.model().await?;
		let staff_role = guild_data.get_staff_role();
		if !user.roles.contains(&staff_role) {
			return Ok(None);
		}

		// Before allowing staff to view the ticket, we need to ensure the staff member has access to the ticket's staff channel.
		// This allows us to do things like have tickets private to administrators (for example).
		match (&ticket.built_in_category, &ticket.custom_category) {
			(Some(category), None) => {
				let db_guild_id = database_id_from_discord_id(guild_id.get());
				let guild: Guild = guilds::table.find(db_guild_id).first(&mut db_connection)?;
				let category_channel = match category {
					BuiltInTicketCategory::BanAppeal => guild.get_ban_appeal_ticket_channel(),
					BuiltInTicketCategory::NewPartner => guild.get_new_partner_ticket_channel(),
					BuiltInTicketCategory::ExistingPartner => guild.get_existing_partner_ticket_channel(),
					BuiltInTicketCategory::MessageReport => guild.get_message_reports_channel(),
				};
				let Some(category_channel) = category_channel else {
					return Ok(None);
				};

				let permissions = channel_permissions(guild_id, category_channel, &discord_client).await;
				let permissions = match permissions {
					Ok(perms) => perms,
					Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
				};
				if !permissions.contains(Permissions::VIEW_CHANNEL) {
					return Ok(None);
				}
			}
			(None, Some(category)) => {
				let custom_category: CustomCategory =
					custom_categories::table.find(category).first(&mut db_connection)?;
				let category_channel = custom_category.get_channel();
				let permissions = channel_permissions(guild_id, category_channel, &discord_client).await;
				let permissions = match permissions {
					Ok(perms) => perms,
					Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
				};
				if !permissions.contains(Permissions::VIEW_CHANNEL) {
					return Ok(None);
				}
			}
			_ => unreachable!(),
		}

		ticket_messages::table
			.filter(ticket_messages::ticket.eq(&ticket_id))
			.load(&mut db_connection)?
	};

	let mut author_ids: HashSet<Id<UserMarker>> = HashSet::new();

	for message in ticket_messages_db.iter() {
		author_ids.insert(message.get_author());
	}

	let mut author_name_tasks: JoinSet<(Id<UserMarker>, String)> = JoinSet::new();
	for author_id in author_ids {
		let discord_client = Arc::clone(&discord_client);
		author_name_tasks.spawn(async move {
			let member = discord_client.guild_member(guild_id, author_id).await.map_err(|_| ());
			let member = match member {
				Ok(member) => member.model().await.map_err(|_| ()),
				Err(error) => Err(error),
			};
			let author_name = member.map(|member| member.nick);
			let author_name = if let Ok(Some(name)) = author_name {
				name
			} else {
				let user = discord_client.user(author_id).await.map_err(|_| ());
				let user = match user {
					Ok(user) => user.model().await.map_err(|_| ()),
					Err(error) => Err(error),
				};
				match user {
					Ok(user) => match user.global_name {
						Some(name) => name,
						None => user.name,
					},
					Err(_) => format!("@{}", author_id.get()),
				}
			};

			(author_id, author_name)
		});
	}

	let author_names = author_name_tasks.join_all().await;
	let author_names: HashMap<Id<UserMarker>, String> = author_names.into_iter().collect();

	let ticket_messages: Vec<TicketMessageWeb> = ticket_messages_db
		.into_iter()
		.map(|message| {
			let author = message.get_author();
			TicketMessageWeb {
				id: message.id.clone(),
				author_name: author_names
					.get(&author)
					.cloned()
					.unwrap_or_else(|| format!("@{}", author.get())),
				send_time: message.send_time,
				internal: message.user_message.is_none(),
				body: message.body,
			}
		})
		.collect();

	Ok(Some(TicketData {
		title: ticket_title,
		category_name,
		messages: ticket_messages,
	}))
}
