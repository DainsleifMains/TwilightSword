// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub const NOT_SET_UP_FOR_GUILD: &str = "This server hasn't yet been set up in Twilight Sword. Use `/setup` to set up.";

pub fn ticket_channel_missing_permissions_message(channel_mention: impl std::fmt::Display) -> String {
	format!("The channel {} does not have the necessary permissions (View Channel, Read Message History, Send Messages, Send Messages in Threads, Create Public Threads, Manage Threads) in the ticket channel to create, update, and manage tickets.", channel_mention)
}
