// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::Form;
use std::collections::HashMap;
use twilight_model::id::Id;
use twilight_model::id::marker::GuildMarker;

#[derive(Debug, Default)]
pub struct FormAssociations {
	pub sessions: HashMap<String, FormAssociationData>,
}

#[derive(Debug)]
pub struct FormAssociationData {
	pub guild_id: Id<GuildMarker>,
	pub all_forms: Vec<Form>,
}
