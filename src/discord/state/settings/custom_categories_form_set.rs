// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::custom_categories_shared::custom_category_list_component;
use crate::model::{CustomCategory, Form};
use std::collections::HashMap;
use twilight_model::channel::message::component::{
	ActionRow, Button, ButtonStyle, Component, SelectMenu, SelectMenuOption, SelectMenuType,
};

#[derive(Debug, Default)]
pub struct CustomCategoryFormAssociations {
	pub sessions: HashMap<String, CustomCategoryFormAssociationData>,
}

#[derive(Debug)]
pub struct CustomCategoryFormAssociationData {
	pub all_categories: Vec<CustomCategory>,
	pub selected_category_id: Option<String>,
	pub current_category_page: usize,
	pub all_forms: Vec<Form>,
	pub selected_form_id: Option<String>,
	pub current_form_page: usize,
}

pub fn custom_category_form_association_components(
	session_id: &str,
	category_list: &[CustomCategory],
	selected_category_id: Option<&String>,
	category_page: usize,
	form_list: &[Form],
	selected_form_id: Option<&String>,
	form_page: usize,
) -> Vec<Component> {
	if category_list.is_empty() || form_list.is_empty() {
		return Vec::new();
	}

	let max_form_page = if form_list.len() <= 25 {
		0
	} else {
		(form_list.len() - 1) / 23
	};
	let form_page_number = form_page.min(max_form_page);
	let mut form_options: Vec<SelectMenuOption> = Vec::with_capacity(25);

	if max_form_page == 0 {
		for form in form_list.iter() {
			form_options.push(form_select_option(form, selected_form_id));
		}
	} else {
		if form_page_number > 0 {
			form_options.push(SelectMenuOption {
				default: false,
				description: Some(String::from("See the previous page of forms")),
				emoji: None,
				label: String::from("Previous Page"),
				value: format!(">{}", form_page_number - 1),
			});
		}
		for form in form_list.iter().skip(form_page_number * 23).take(23) {
			form_options.push(form_select_option(form, selected_form_id));
		}
		if form_page_number < max_form_page {
			form_options.push(SelectMenuOption {
				default: false,
				description: Some(String::from("See the next page of forms")),
				emoji: None,
				label: String::from("Next Page"),
				value: format!(">{}", form_page_number + 1),
			});
		}
	}

	let category_component = custom_category_list_component(
		"custom_categories_form_set",
		session_id,
		category_list,
		selected_category_id,
		category_page,
	);

	let form_menu = SelectMenu {
		channel_types: None,
		custom_id: format!("settings/custom_categories_form_set/{}/form", session_id),
		default_values: None,
		disabled: false,
		kind: SelectMenuType::Text,
		max_values: None,
		min_values: None,
		options: Some(form_options),
		placeholder: Some(String::from("Form")),
	};
	let form_component = Component::ActionRow(ActionRow {
		components: vec![Component::SelectMenu(form_menu)],
	});

	let submit_button = Button {
		custom_id: Some(format!("settings/custom_categories_form_set/{}/submit", session_id)),
		disabled: selected_category_id.is_none() || selected_form_id.is_none(),
		emoji: None,
		label: Some(String::from("Set Form")),
		style: ButtonStyle::Primary,
		url: None,
		sku_id: None,
	};
	let submit_component = Component::ActionRow(ActionRow {
		components: vec![Component::Button(submit_button)],
	});

	vec![category_component, form_component, submit_component]
}

fn form_select_option(form: &Form, selected_id: Option<&String>) -> SelectMenuOption {
	let default = match selected_id {
		Some(selection) => form.id == *selection,
		None => false,
	};
	SelectMenuOption {
		default,
		description: None,
		emoji: None,
		label: form.title.clone(),
		value: form.id.clone(),
	}
}
