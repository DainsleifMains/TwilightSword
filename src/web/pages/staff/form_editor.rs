// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::web::pages::utils::FormParams;
use leptos::ev::{MouseEvent, SubmitEvent};
use leptos::prelude::*;
use leptos::task::spawn;
use leptos_router::hooks::{use_navigate, use_params};
use reactive_stores::Store;
use serde::{Deserialize, Serialize};

#[component]
pub fn FormEditor() -> impl IntoView {
	let params = use_params::<FormParams>();
	let guild_id = params.read().as_ref().ok().and_then(|params| params.guild);
	let form_id = params.read().as_ref().ok().and_then(|params| params.form_id.clone());

	let form = OnceResource::new(get_form_data_for_resource(guild_id, form_id));

	let (form_title, set_form_title) = signal(String::new());
	let form_questions = Store::new(FormDataQuestionList::default());
	let (submit_errors, set_submit_errors): (ReadSignal<Vec<String>>, WriteSignal<Vec<String>>) = signal(Vec::new());

	Effect::new_isomorphic(move |_| {
		if let Some(Ok(Some(form_data))) = form.read().as_ref() {
			set_form_title.set(form_data.title.clone());
			form_questions
				.question_list()
				.set(form_data.questions.question_list.clone());
		}
	});

	Effect::new(move |_| {
		if !form_questions.with(|question_list| {
			question_list
				.question_list
				.is_sorted_by_key(|question| question.form_position)
		}) {
			form_questions
				.question_list()
				.update(|question_list| question_list.sort_by_key(|question| question.form_position));
		}
	});

	let add_question_button_click = move |_: MouseEvent| {
		form_questions.question_list().update(|questions| {
			let form_position = questions.last().map(|question| question.form_position + 1).unwrap_or(1);
			let new_question = FormDataQuestion {
				id: String::new(),
				form_position,
				question: String::new(),
			};
			questions.push(new_question);
		});
	};

	let form_submit = move |event: SubmitEvent| {
		event.prevent_default();
		set_submit_errors.set(Vec::new());

		if form_title.with(|title| title.is_empty()) {
			set_submit_errors.update(|errors| errors.push(String::from("Form must have a title")));
		}

		let questions = form_questions.get();
		for question in questions.question_list.iter() {
			if question.question.is_empty() {
				set_submit_errors.update(|errors| errors.push(String::from("All questions must have prompt text")));
				break;
			}
		}
		if questions.question_list.is_empty() {
			set_submit_errors.update(|errors| errors.push(String::from("Form must have questions")));
		}

		if !submit_errors.with(|errors| errors.is_empty()) {
			return;
		}

		let form_id = match form.read().as_ref() {
			Some(Ok(Some(form_data))) => form_data.id.clone(),
			_ => String::new(),
		};

		let form_questions: Vec<FormDataQuestion> = form_questions
			.get()
			.question_list
			.into_iter()
			.map(|question| FormDataQuestion {
				id: question.id,
				form_position: question.form_position,
				question: question.question,
			})
			.collect();

		let form_data = FormData {
			id: form_id,
			title: form_title.get(),
			questions: FormDataQuestionList {
				question_list: form_questions,
			},
		};

		spawn(async move {
			let _ = update_form_data(guild_id, form_data).await;

			let navigate_destination = match guild_id {
				Some(id) => format!("/{}/staff/manage_forms", id),
				None => String::from("/staff/manage_forms"),
			};
			use_navigate()(&navigate_destination, Default::default());
		});
	};

	view! {
		<h2>"Edit Form"</h2>
		<Transition>
			{form.read();}
			<form on:submit=form_submit>
				<div class="form_manager_submit_errors">
					<ul>
						<For
							each=move || submit_errors.get()
							key=|error| error.clone()
							let(error)
						>
							<li>{error}</li>
						</For>
					</ul>
				</div>
				<div>
					<label>
						<span class="form_manager_label_text">
							"Form Title"
						</span>
						<input
							type="text"
							class="form_manager_title_input"
							bind:value=(form_title, set_form_title)
						/>
					</label>
				</div>
				<div>
					<h3>"Questions"</h3>
					<For
						each=move || form_questions.question_list()
						key=|question| question.read().id.clone()
						children=|question| {
							let (question_prompt, set_question_prompt) = signal(question.read().question.clone());
							Effect::new(move |_| {
								let prompt = question_prompt.get();
								question.update(|question| question.question = prompt);
							});
							view! {
								<div>
									<input
										class="form_manager_question_prompt"
										bind:value=(question_prompt, set_question_prompt)
									/>
								</div>
							}
						}
					/>
				</div>
				<div>
					<button type="button" on:click=add_question_button_click>
						"Add Question"
					</button>
				</div>
				<div class="form_manager_save_button">
					<button type="submit">Save Changes</button>
				</div>
			</form>
		</Transition>
	}
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FormData {
	id: String,
	title: String,
	questions: FormDataQuestionList,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Store)]
pub struct FormDataQuestionList {
	#[store(key: String = |question| question.id.clone())]
	question_list: Vec<FormDataQuestion>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Store)]
pub struct FormDataQuestion {
	id: String,
	form_position: i32,
	question: String,
}

async fn get_form_data_for_resource(
	guild_id: Option<u64>,
	form_id: Option<String>,
) -> Result<Option<FormData>, ServerFnError> {
	if let Some(form_id) = form_id {
		let result = get_form_data(guild_id, form_id).await;
		result.map(Some)
	} else {
		Ok(None)
	}
}

#[server]
async fn get_form_data(guild_id: Option<u64>, form_id: String) -> Result<FormData, ServerFnError> {
	use crate::model::{Form, FormQuestion, Guild, database_id_from_discord_id};
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

	let form: Form = forms::table.find(&form_id).first(&mut db_connection)?;
	let form_questions: Vec<FormQuestion> = form_questions::table
		.filter(form_questions::form.eq(&form.id))
		.load(&mut db_connection)?;

	let form = FormData {
		id: form.id,
		title: form.title,
		questions: FormDataQuestionList {
			question_list: form_questions
				.into_iter()
				.map(|question| FormDataQuestion {
					id: question.id,
					form_position: question.form_position,
					question: question.question,
				})
				.collect(),
		},
	};

	Ok(form)
}

#[server]
async fn update_form_data(guild_id: Option<u64>, form: FormData) -> Result<(), ServerFnError> {
	use crate::model::{Form, FormQuestion, Guild, database_id_from_discord_id};
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

	if form.title.is_empty() {
		return Err(ServerFnError::ServerError(String::from("Form must have a title")));
	}
	if form.questions.question_list.is_empty() {
		return Err(ServerFnError::ServerError(String::from(
			"Form must have at least one question",
		)));
	}

	let mut save_form = Form {
		id: form.id,
		guild: db_guild_id,
		title: form.title,
	};
	if save_form.id.is_empty() {
		save_form.id = cuid2::create_id();
	} else {
		let form: Form = forms::table.find(&save_form.id).first(&mut db_connection)?;
		if form.guild != db_guild_id {
			return Err(ServerFnError::ServerError(String::from("Invalid form")));
		}
	}
	let questions: Vec<FormQuestion> = form
		.questions
		.question_list
		.into_iter()
		.map(|question| FormQuestion {
			id: if question.id.is_empty() {
				cuid2::create_id()
			} else {
				question.id
			},
			form: save_form.id.clone(),
			form_position: question.form_position,
			question: question.question,
		})
		.collect();

	db_connection.transaction(|db_connection| {
		diesel::delete(form_questions::table)
			.filter(form_questions::form.eq(&save_form.id))
			.execute(db_connection)?;

		let form_title = save_form.title.clone();
		diesel::insert_into(forms::table)
			.values(save_form)
			.on_conflict(forms::id)
			.do_update()
			.set(forms::title.eq(form_title))
			.execute(db_connection)?;

		diesel::insert_into(form_questions::table)
			.values(questions)
			.execute(db_connection)?;

		Ok(())
	})
}
