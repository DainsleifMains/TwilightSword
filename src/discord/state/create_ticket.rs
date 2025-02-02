// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::Guild;

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

	pub fn to_id(&self) -> &'static str {
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

	pub fn user_can_submit(&self) -> bool {
		matches!(self, Self::BanAppeal | Self::NewPartner | Self::ExistingPartner)
	}

	pub fn name(&self) -> &'static str {
		match self {
			Self::BanAppeal => "Ban Appeal",
			Self::NewPartner => "New Partner",
			Self::ExistingPartner => "Existing Partner",
			Self::MessageReport => "Message Report",
		}
	}

	pub fn is_enabled_for_guild(&self, guild: &Guild) -> bool {
		match self {
			Self::BanAppeal => guild.ban_appeal_ticket_channel.is_some(),
			Self::NewPartner => guild.new_partner_ticket_channel.is_some(),
			Self::ExistingPartner => guild.existing_partner_ticket_channel.is_some(),
			Self::MessageReport => guild.message_reports_channel.is_some(),
		}
	}
}
