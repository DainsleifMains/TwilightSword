// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::Ticket;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ReplyStates {
	pub states: HashMap<String, ReplyState>,
}

#[derive(Debug)]
pub struct ReplyState {
	pub ticket: Ticket,
}
