// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::web::pages::utils::GuildParam;
use chrono::{DateTime, Utc};
use leptos::prelude::*;
use leptos_router::hooks::use_params;
use serde::{Deserialize, Serialize};

#[component]
pub fn Dashboard() -> impl IntoView {
	let params = use_params::<GuildParam>();
	let guild_id = params.read().as_ref().ok().and_then(|params| params.guild);

	let user_tickets = Resource::new(|| (), move |_| get_active_tickets_for_user(guild_id));

	view! {
		<Transition fallback=|| view! { <div class="dashboard_ticket_list_loading">"Loading tickets..."</div> }>
			<table class="dashboard_ticket_list">
				<thead>
					<tr>
						<th>Ticket</th>
						<th>Last Message Author</th>
						<th>Last Message Time</th>
					</tr>
				</thead>
				<tbody>
					{
						move || match &user_tickets.read().as_ref().and_then(|tickets| tickets.as_ref().ok()) {
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
	}
}

fn make_ticket_url(guild_id: Option<u64>, ticket_id: &str) -> String {
	match guild_id {
		Some(id) => format!("/{}/ticket/{}", id, ticket_id),
		None => format!("/ticket/{}", ticket_id),
	}
}

#[derive(Deserialize, Serialize)]
pub struct TicketMetadata {
	id: String,
	title: String,
	last_message_author_name: String,
	last_message_time: DateTime<Utc>,
}

#[server]
async fn get_active_tickets_for_user(guild_id: Option<u64>) -> Result<Vec<TicketMetadata>, ServerFnError> {
	use crate::discord::users::get_member_data;
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

	let mut tickets: Vec<TicketMetadata> = Vec::with_capacity(user_tickets.len());
	for ticket in user_tickets {
		let last_message: TicketMessage = ticket_messages::table
			.filter(ticket_messages::ticket.eq(&ticket.id).and(ticket_messages::user_message.is_not_null()))
			.order(ticket_messages::send_time.desc())
			.first(&mut db_connection)?;
		let author_id = last_message.get_author();

		let author_data = get_member_data(&state.discord_client, guild_id, author_id).await;
		let author_name = match author_data {
			Ok(data) => data.display_name,
			Err(_) => format!("<{}>", author_id.get()),
		};

		let ticket_metadata = TicketMetadata {
			id: ticket.id,
			title: ticket.title,
			last_message_author_name: author_name,
			last_message_time: last_message.send_time,
		};
		tickets.push(ticket_metadata);
	}
	Ok(tickets)
}
