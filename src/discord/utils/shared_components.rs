// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle, Component};

pub fn new_ticket_button() -> Component {
	let create_button_id = String::from("create_ticket");
	let create_button = Button {
		custom_id: Some(create_button_id),
		disabled: false,
		emoji: None,
		label: Some(String::from("Create Ticket")),
		style: ButtonStyle::Primary,
		url: None,
		sku_id: None,
	};
	Component::ActionRow(ActionRow {
		components: vec![Component::Button(create_button)],
	})
}
