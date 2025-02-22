// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::web::pages::utils::GuildParam;
use leptos::prelude::*;
use leptos_router::hooks::use_params;
use serde::{Deserialize, Serialize};

#[component]
pub fn Dashboard() -> impl IntoView {
	let params = use_params::<GuildParam>();
	let guild_id = params.read().as_ref().ok().and_then(|params| params.guild);

	let user_tickets = Resource::new(|| (), move |_| get_user_tickets(guild_id));

	view! {
		<Transition fallback=|| view! { <div id="dashboard_ticket_list_loading">"Loading tickets..."</div> }>
			<table id="dashboard_ticket_list">
				<thead>
					<tr>
						<th>Ticket</th>
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
										</tr>
									}.into_any()
								).collect::<Vec<_>>()
							}
							_ => {
								let no_tickets_view = view! {
									<tr>
										<td colspan={1} class="dashboard_ticket_list_no_tickets">
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
}

#[server]
async fn get_user_tickets(guild_id: Option<u64>) -> Result<Vec<TicketMetadata>, ServerFnError> {
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
				.and(tickets::closed_at.is_null()),
		)
		.load(&mut db_connection)?;

	let mut tickets: Vec<TicketMetadata> = Vec::with_capacity(user_tickets.len());
	for ticket in user_tickets {
		let ticket_metadata = TicketMetadata {
			id: ticket.id,
			title: ticket.title,
		};
		tickets.push(ticket_metadata);
	}
	Ok(tickets)
}
