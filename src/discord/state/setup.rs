// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use twilight_model::channel::message::component::{
	ActionRow, Button, ButtonStyle, Component, SelectMenu, SelectMenuType,
};
use twilight_model::id::marker::{GuildMarker, RoleMarker};
use twilight_model::id::Id;

#[derive(Debug, Default)]
pub struct SetupState {
	pub states: HashMap<String, SetupInstance>,
}

#[derive(Debug)]
pub struct SetupInstance {
	pub admin_role: Option<Id<RoleMarker>>,
	pub staff_role: Option<Id<RoleMarker>>,
	pub guild: Id<GuildMarker>,
	pub initial_message_token: String,
}

impl SetupInstance {
	pub fn new(guild: Id<GuildMarker>, initial_message_token: String) -> Self {
		Self {
			admin_role: None,
			staff_role: None,
			guild,
			initial_message_token,
		}
	}
}

pub fn set_up_components(setup_id: &str, confirm_button_disabled: bool) -> Vec<Component> {
	let admin_role_select_id = format!("setup/{}/admin_role", setup_id);
	let staff_role_select_id = format!("setup/{}/staff_role", setup_id);
	let set_up_button_id = format!("setup/{}/confirm", setup_id);
	let cancel_button_id = format!("setup/{}/cancel", setup_id);

	let admin_role_select = SelectMenu {
		kind: SelectMenuType::Role,
		custom_id: admin_role_select_id,
		placeholder: Some(String::from("Admin Role")),
		channel_types: None,
		default_values: None,
		disabled: false,
		min_values: None,
		max_values: None,
		options: None,
	};
	let staff_role_select = SelectMenu {
		kind: SelectMenuType::Role,
		custom_id: staff_role_select_id,
		placeholder: Some(String::from("Staff Role")),
		channel_types: None,
		default_values: None,
		disabled: false,
		min_values: None,
		max_values: None,
		options: None,
	};
	let set_up_button = Button {
		label: Some(String::from("Set Up!")),
		style: ButtonStyle::Primary,
		custom_id: Some(set_up_button_id),
		disabled: confirm_button_disabled,
		emoji: None,
		url: None,
	};
	let cancel_button = Button {
		label: Some(String::from("Cancel")),
		style: ButtonStyle::Secondary,
		custom_id: Some(cancel_button_id),
		disabled: false,
		emoji: None,
		url: None,
	};

	let admin_role_row = ActionRow {
		components: vec![Component::SelectMenu(admin_role_select)],
	};
	let staff_role_row = ActionRow {
		components: vec![Component::SelectMenu(staff_role_select)],
	};
	let buttons_row = ActionRow {
		components: vec![Component::Button(set_up_button), Component::Button(cancel_button)],
	};

	vec![
		Component::ActionRow(admin_role_row),
		Component::ActionRow(staff_role_row),
		Component::ActionRow(buttons_row),
	]
}
