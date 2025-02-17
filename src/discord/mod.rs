// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod commands;
mod connection;
mod events;
mod incoming_messages;
mod interactions;
mod state;
mod utils;

pub use connection::{run_bot, set_up_client};
pub use utils::tickets::MAX_TICKET_TITLE_LENGTH;
