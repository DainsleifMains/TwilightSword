// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::{BuiltInTicketCategory, Guild};
use std::collections::HashMap;
use std::fmt;
use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle, Component};

#[derive(Clone, Copy, Debug)]
pub enum BuiltInCategory {
	BanAppeal,
	NewPartner,
	ExistingPartner,
	MessageReport,
}

impl BuiltInCategory {
	pub fn from_id(id: &str) -> Option<Self> {
		match id {
			"1" => Some(Self::BanAppeal),
			"2" => Some(Self::NewPartner),
			"3" => Some(Self::ExistingPartner),
			"4" => Some(Self::MessageReport),
			_ => None,
		}
	}

	pub fn as_id(&self) -> &'static str {
		match self {
			Self::BanAppeal => "1",
			Self::NewPartner => "2",
			Self::ExistingPartner => "3",
			Self::MessageReport => "4",
		}
	}

	pub fn all_categories() -> Vec<Self> {
		vec![
			Self::BanAppeal,
			Self::NewPartner,
			Self::ExistingPartner,
			Self::MessageReport,
		]
	}

	pub fn user_can_submit_from_server(&self) -> bool {
		matches!(self, Self::NewPartner | Self::ExistingPartner)
	}

	pub fn is_enabled_for_guild(&self, guild: &Guild) -> bool {
		match self {
			Self::BanAppeal => guild.ban_appeal_ticket_channel.is_some(),
			Self::NewPartner => guild.new_partner_ticket_channel.is_some(),
			Self::ExistingPartner => guild.existing_partner_ticket_channel.is_some(),
			Self::MessageReport => guild.message_reports_channel.is_some(),
		}
	}

	pub fn to_database(self) -> BuiltInTicketCategory {
		match self {
			Self::BanAppeal => BuiltInTicketCategory::BanAppeal,
			Self::NewPartner => BuiltInTicketCategory::NewPartner,
			Self::ExistingPartner => BuiltInTicketCategory::ExistingPartner,
			Self::MessageReport => BuiltInTicketCategory::MessageReport,
		}
	}
}

impl fmt::Display for BuiltInCategory {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let db_category = self.to_database();
		write!(f, "{}", db_category)
	}
}

#[derive(Debug, Default)]
pub struct CreateTicketStates {
	pub states: HashMap<String, CreateTicketState>,
}

#[derive(Debug, Default)]
pub struct CreateTicketState {
	pub built_in_category: Option<BuiltInCategory>,
	pub custom_category_id: Option<String>,
}

pub fn new_ticket_button() -> Component {
	let create_button_id = String::from("create_ticket//start");
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
