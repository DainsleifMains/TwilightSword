// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::Form;
use twilight_model::channel::message::component::{
	ActionRow, Button, ButtonStyle, Component, SelectMenu, SelectMenuOption, SelectMenuType,
};

pub fn ticket_form_association_components(
	ticket_type_identifier: &str,
	session_id: &str,
	form_list: &[Form],
	selected_id: Option<&String>,
	page_number: usize,
) -> Vec<Component> {
	if form_list.is_empty() {
		return Vec::new();
	}
	let max_page = if form_list.len() <= 25 {
		0
	} else {
		(form_list.len() - 1) / 23
	};

	let page_number = page_number.max(max_page);

	let mut select_options: Vec<SelectMenuOption> = Vec::with_capacity(25);
	if max_page == 0 {
		for form in form_list.iter() {
			select_options.push(form_select_option(form, selected_id));
		}
	} else {
		if page_number > 0 {
			select_options.push(SelectMenuOption {
				default: false,
				description: Some(String::from("See the previous page of forms")),
				emoji: None,
				label: String::from("Previous Page"),
				value: format!(">{}", page_number - 1),
			});
		}
		for form in form_list.iter().skip(page_number * 23).take(23) {
			select_options.push(form_select_option(form, selected_id));
		}
		if page_number < max_page {
			select_options.push(SelectMenuOption {
				default: false,
				description: Some(String::from("See the next page of forms")),
				emoji: None,
				label: String::from("Next Page"),
				value: format!(">{}", page_number + 1),
			});
		}
	}

	let select_menu = SelectMenu {
		channel_types: None,
		custom_id: format!("settings/{}/{}/form", ticket_type_identifier, session_id),
		default_values: None,
		disabled: false,
		kind: SelectMenuType::Text,
		max_values: None,
		min_values: None,
		options: Some(select_options),
		placeholder: Some(String::from("Form")),
	};
	let form_component = Component::ActionRow(ActionRow {
		components: vec![Component::SelectMenu(select_menu)],
	});

	let submit_button = Button {
		custom_id: Some(format!("settings/{}/{}/submit", ticket_type_identifier, session_id)),
		disabled: selected_id.is_none(),
		emoji: None,
		label: Some(String::from("Set Form")),
		style: ButtonStyle::Primary,
		url: None,
		sku_id: None,
	};
	let submit_component = Component::ActionRow(ActionRow {
		components: vec![Component::Button(submit_button)],
	});

	vec![form_component, submit_component]
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
