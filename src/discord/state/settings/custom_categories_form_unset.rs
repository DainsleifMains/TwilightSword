// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::custom_categories_shared::custom_category_list_component;
use crate::model::CustomCategory;
use std::collections::HashMap;
use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle, Component};

#[derive(Debug, Default)]
pub struct CustomCategoryFormRemovals {
	pub sessions: HashMap<String, CustomCategoryFormRemovalData>,
}

#[derive(Debug)]
pub struct CustomCategoryFormRemovalData {
	pub all_categories: Vec<CustomCategory>,
	pub selected_id: Option<String>,
	pub current_page: usize,
}

pub fn custom_category_form_removal_components(
	session_id: &str,
	category_list: &[CustomCategory],
	selected_id: Option<&String>,
	current_page: usize,
) -> Vec<Component> {
	if category_list.is_empty() {
		return Vec::new();
	}

	let category_component = custom_category_list_component(
		"custom_categories_form_unset",
		session_id,
		category_list,
		selected_id,
		current_page,
	);

	let submit_button = Button {
		custom_id: Some(format!("settings/custom_categories_form_unset/{}/submit", session_id)),
		disabled: selected_id.is_none(),
		emoji: None,
		label: Some(String::from("Remove Form")),
		style: ButtonStyle::Primary,
		url: None,
		sku_id: None,
	};
	let submit_component = Component::ActionRow(ActionRow {
		components: vec![Component::Button(submit_button)],
	});

	vec![category_component, submit_component]
}
