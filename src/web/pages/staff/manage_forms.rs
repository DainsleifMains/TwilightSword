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
pub fn ManageForms() -> impl IntoView {
	let params = use_params::<GuildParam>();
	let guild_id = params.read().as_ref().ok().and_then(|params| params.guild);

	let forms = Resource::new(|| (), move |_| get_guild_forms(guild_id));

	view! {
		<h2>All Forms</h2>
		<Transition>
			{
				move || match &forms.read().as_ref().and_then(|forms| forms.as_ref().ok()) {
					Some(forms) if !forms.is_empty() => {
						view! {
							<ul class="form_list">
								{
									forms.iter().map(|form|
										view! {
											<li>
												<a href={make_form_edit_url(guild_id, &form.id)}>
													{form.title.clone()}
												</a>
											</li>
										}.into_any()
									).collect::<Vec<_>>()
								}
							</ul>
						}.into_any()
					}
					_ => view! {
						<div class="manage_forms_none">No forms have been created.</div>
					}.into_any()
				}
			}
		</Transition>
		<div class="manage_forms_create_link">
			<a href={make_form_edit_url(guild_id, "new")}>
				"Create new form"
			</a>
		</div>
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FormData {
	pub id: String,
	pub title: String,
	pub questions: Vec<FormQuestion>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FormQuestion {
	pub id: String,
	pub question: String,
	pub form_position: i32,
}

#[server]
async fn get_guild_forms(guild_id: Option<u64>) -> Result<Vec<FormData>, ServerFnError> {
	use crate::model::{Form, FormQuestion as FormQuestionDb, Guild, database_id_from_discord_id};
	use crate::schema::{form_questions, forms, guilds};
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

	let staff_role = guild.get_staff_role();

	let member = state
		.discord_client
		.guild_member(guild_id, user_id)
		.await?
		.model()
		.await?;

	if !member.roles.contains(&staff_role) {
		return Err(ServerFnError::ServerError(String::from("Permission denied")));
	}

	let guild_forms: Vec<Form> = forms::table
		.filter(forms::guild.eq(db_guild_id))
		.load(&mut db_connection)?;
	let form_ids: Vec<&str> = guild_forms.iter().map(|form| form.id.as_str()).collect();
	let mut guild_form_questions: Vec<FormQuestionDb> = form_questions::table
		.filter(form_questions::form.eq_any(&form_ids))
		.load(&mut db_connection)?;

	let mut form_list: Vec<FormData> = Vec::with_capacity(guild_forms.len());
	for form in guild_forms {
		let mut new_guild_questions: Vec<FormQuestionDb> = Vec::new();
		let mut form_questions: Vec<FormQuestion> = Vec::new();

		for question in guild_form_questions {
			if question.form == form.id {
				form_questions.push(FormQuestion {
					id: question.id,
					question: question.question,
					form_position: question.form_position,
				});
			} else {
				new_guild_questions.push(question);
			}
		}

		guild_form_questions = new_guild_questions;

		form_list.push(FormData {
			id: form.id,
			title: form.title,
			questions: form_questions,
		});
	}

	Ok(form_list)
}

/// Makes the edit URL for a form
fn make_form_edit_url(guild_id: Option<u64>, form_id: &str) -> String {
	match guild_id {
		Some(id) => format!("/{}/staff/edit_form/{}", id, form_id),
		None => format!("/staff/edit_form/{}", form_id),
	}
}
