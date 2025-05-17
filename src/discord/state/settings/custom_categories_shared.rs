// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::CustomCategory;
use twilight_model::channel::message::component::{ActionRow, Component, SelectMenu, SelectMenuOption, SelectMenuType};

/// Creates a component for a list of custom categories.
///
/// Assumes that there are categories in the list. If the list is empty, this will create a component that Discord will
/// reject as invalid (due to having no options).
pub fn custom_category_list_component(
	interaction_type_name: &str,
	session_id: &str,
	category_list: &[CustomCategory],
	selected_category_id: Option<&String>,
	category_page: usize,
) -> Component {
	let max_category_page = if category_list.len() <= 25 {
		0
	} else {
		(category_list.len() - 1) / 23
	};
	let category_page_number = category_page.min(max_category_page);

	let mut category_options: Vec<SelectMenuOption> = Vec::with_capacity(25);

	if max_category_page == 0 {
		for category in category_list.iter() {
			category_options.push(category_select_option(category, selected_category_id));
		}
	} else {
		if category_page_number > 0 {
			category_options.push(SelectMenuOption {
				default: false,
				description: Some(String::from("See the previous page of categories")),
				emoji: None,
				label: String::from("Previous Page"),
				value: format!(">{}", category_page_number - 1),
			});
		}
		for category in category_list.iter().skip(category_page_number * 23).take(23) {
			category_options.push(category_select_option(category, selected_category_id));
		}
		if category_page_number < max_category_page {
			category_options.push(SelectMenuOption {
				default: false,
				description: Some(String::from("See the next page of categories")),
				emoji: None,
				label: String::from("Next Page"),
				value: format!(">{}", category_page_number + 1),
			});
		}
	}

	let category_menu = SelectMenu {
		channel_types: None,
		custom_id: format!("settings/{}/{}/category", interaction_type_name, session_id),
		default_values: None,
		disabled: false,
		kind: SelectMenuType::Text,
		max_values: None,
		min_values: None,
		options: Some(category_options),
		placeholder: Some(String::from("Category")),
	};

	Component::ActionRow(ActionRow {
		components: vec![Component::SelectMenu(category_menu)],
	})
}

fn category_select_option(category: &CustomCategory, selected_id: Option<&String>) -> SelectMenuOption {
	let default = match selected_id {
		Some(selection) => category.id == *selection,
		None => false,
	};
	SelectMenuOption {
		default,
		description: None,
		emoji: None,
		label: category.name.clone(),
		value: category.id.clone(),
	}
}
