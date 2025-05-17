// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::custom_categories_shared::custom_category_list_component;
use crate::model::CustomCategory;
use std::collections::HashMap;
use twilight_model::channel::message::component::Component;

#[derive(Debug, Default)]
pub struct CustomCategoryFormRetrievals {
	pub sessions: HashMap<String, CustomCategoryFormRetrievalData>,
}

#[derive(Debug)]
pub struct CustomCategoryFormRetrievalData {
	pub all_categories: Vec<CustomCategory>,
	pub current_page: usize,
}

pub fn custom_category_form_retrieval_components(
	session_id: &str,
	category_list: &[CustomCategory],
	selected_id: Option<&String>,
	current_page: usize,
) -> Vec<Component> {
	if category_list.is_empty() {
		return Vec::new();
	}

	let category_component = custom_category_list_component(
		"custom_categories_form_get",
		session_id,
		category_list,
		selected_id,
		current_page,
	);
	vec![category_component]
}
