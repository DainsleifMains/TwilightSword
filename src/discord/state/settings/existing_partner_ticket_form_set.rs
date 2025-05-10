// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::ticket_form_set::ticket_form_association_components;
use crate::model::Form;
use std::collections::HashMap;
use twilight_model::channel::message::component::Component;
use twilight_model::id::Id;
use twilight_model::id::marker::GuildMarker;

#[derive(Debug, Default)]
pub struct ExistingPartnerFormAssociations {
	pub sessions: HashMap<String, ExistingPartnerFormAssociationData>,
}

#[derive(Debug)]
pub struct ExistingPartnerFormAssociationData {
	pub guild_id: Id<GuildMarker>,
	pub all_forms: Vec<Form>,
	pub selected_form_id: Option<String>,
	pub current_page: usize,
}

/// Gets the message component data that should be used currently in the message containing the components
pub fn existing_partner_form_association_components(
	session_id: &str,
	form_list: &[Form],
	selected_id: Option<&String>,
	page_number: usize,
) -> Vec<Component> {
	ticket_form_association_components(
		"existing_partner_ticket_form_set",
		session_id,
		form_list,
		selected_id,
		page_number,
	)
}
