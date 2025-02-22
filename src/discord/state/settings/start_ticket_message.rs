// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use twilight_model::id::Id;
use twilight_model::id::marker::GuildMarker;

#[derive(Debug, Default)]
pub struct StartTicketMessageState {
	pub guilds: HashMap<String, Id<GuildMarker>>,
}
