// © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::schema::{custom_categories, form_questions, forms, guilds, tickets};
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, InteractionMarker, RoleMarker, UserMarker};
use twilight_model::id::Id;

#[derive(DbEnum, Debug)]
#[ExistingTypePath = "crate::schema::sql_types::BuiltInTicketCategory"]
pub enum BuiltInTicketCategory {
	BanAppeal,
	NewPartner,
	ExistingPartner,
	MessageReport,
}

/// Gets a guild that's using the bot and its various settings.
#[derive(Default, Insertable, Queryable)]
pub struct Guild {
	/// The ID of the guild in question.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_guild_id].
	pub guild_id: i64,
	/// The ID of the channel in which the "open a ticket here!" message is sent.
	/// If the feature is disabled, no ID will be entered.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_start_ticket_channel].
	pub start_ticket_channel: Option<i64>,
	/// The message used in the "open a ticket" channel.
	pub start_ticket_message: String,
	/// The interaction ID of the active message posted by the bot for starting a ticket.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_start_ticket_interaction].
	pub start_ticket_interaction: Option<i64>,
	/// The token used with the Discord API for updating the start ticket message.
	pub start_ticket_token: Option<String>,
	/// The ID of the channel to which ban appeal tickets are sent.
	/// If the feature is disabled, no ID will be entered.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_ban_appeal_ticket_channel].
	pub ban_appeal_ticket_channel: Option<i64>,
	/// The ID of the channel to which new partnership tickets are sent.
	/// If the feature is disabled, no ID will be entered.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_new_partner_ticket_channel].
	pub new_partner_ticket_channel: Option<i64>,
	/// The ID of the channel to which existing partnership tickets are sent.
	/// If the feature is disabled, no ID will be entered.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_existing_partner_ticket_channel].
	pub existing_partner_ticket_channel: Option<i64>,
	/// The ID of the channel to which message reports are sent.
	/// If the feature is disabled, no ID will be entered.
	///
	/// For the raw database representation, use [Self::get_message_reports_channel].
	pub message_reports_channel: Option<i64>,
	/// Whether TCN partner integration is enabled for this guild.
	pub tcn_partner_integration: bool,
	/// The ID of the role administrators have.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_admin_role].
	pub admin_role: i64,
	/// The ID of the role all staff have.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_staff_role].
	pub staff_role: i64,
	/// The ID of the channel in which the bot will complain about reasons for moderator actions.
	/// If the feature is disabled, no ID will be entered.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_action_reason_complain_channel].
	pub action_reason_complain_channel: Option<i64>,
	/// The ID of the form used for ban appeal tickets, if those tickets use a form.
	pub ban_appeal_ticket_form: Option<String>,
	/// The ID of the form used for new partnership request tickets, if those tickets use a form.
	pub new_partner_ticket_form: Option<String>,
	/// The ID of the form used for existing partnership tickets, if those tickets use a form.
	pub existing_partner_ticket_form: Option<String>,
}

impl Guild {
	/// Gets the Discord-facing guild information.
	///
	/// For the raw database representation, use [Self::guild_id].
	pub fn get_guild_id(&self) -> Id<GuildMarker> {
		Id::new(discord_id_from_database_id(self.guild_id))
	}

	/// Gets the channel in which the "open a ticket here!" message is sent.
	/// If the feature is disabled, no channel will be returned.
	///
	/// For the raw database representation, use [Self::start_ticket_channel].
	pub fn get_start_ticket_channel(&self) -> Option<Id<ChannelMarker>> {
		self.start_ticket_channel
			.map(|database_id| Id::new(discord_id_from_database_id(database_id)))
	}

	/// If a message is posted to the start ticket channel, the interaction ID of that message.
	///
	/// For the raw database representation, use [Self::start_ticket_integration].
	pub fn get_start_ticket_interaction(&self) -> Option<Id<InteractionMarker>> {
		self.start_ticket_interaction
			.map(|database_id| Id::new(discord_id_from_database_id(database_id)))
	}

	/// Gets the channel to which ban appeal tickets are sent.
	/// If the feature is disabled, no channel will be returned.
	///
	/// For the raw database representation, use [Self::ban_appeal_ticket_channel].
	pub fn get_ban_appeal_ticket_channel(&self) -> Option<Id<ChannelMarker>> {
		self.ban_appeal_ticket_channel
			.map(|database_id| Id::new(discord_id_from_database_id(database_id)))
	}

	/// Gets the channel to which new partnership request tickets are sent.
	/// If the feature is disabled, no channel will be returned.
	///
	/// For the raw database representation, use [Self::new_partner_ticket_channel].
	pub fn get_new_partner_ticket_channel(&self) -> Option<Id<ChannelMarker>> {
		self.new_partner_ticket_channel
			.map(|database_id| Id::new(discord_id_from_database_id(database_id)))
	}

	/// Gets the channel to which existing partnership tickets are sent.
	/// If the feature is disabled, no channel will be returned.
	///
	/// For the raw database representation, use [Self::existing_partner_ticket_channel].
	pub fn get_existing_partner_ticket_channel(&self) -> Option<Id<ChannelMarker>> {
		self.existing_partner_ticket_channel
			.map(|database_id| Id::new(discord_id_from_database_id(database_id)))
	}

	/// Gets the channel to which message report tickets are sent.
	/// If the feature is disabled, no channel will be returned.
	///
	/// For the raw database representation, use [Self::message_reports_channel].
	pub fn get_message_reports_channel(&self) -> Option<Id<ChannelMarker>> {
		self.message_reports_channel
			.map(|database_id| Id::new(discord_id_from_database_id(database_id)))
	}

	/// Gets the role that administrators have.
	///
	/// For the raw database representation, use [Self::admin_role].
	pub fn get_admin_role(&self) -> Id<RoleMarker> {
		Id::new(discord_id_from_database_id(self.admin_role))
	}

	/// Gets the role that all staff have.
	///
	/// For the raw database representation, use [Self::staff_role].
	pub fn get_staff_role(&self) -> Id<RoleMarker> {
		Id::new(discord_id_from_database_id(self.staff_role))
	}

	/// Gets the channel in which the bot complains about moderator action reasons.
	/// If the feature is disabled, no channel will be returned.
	///
	/// For the raw database representation, use [Self::action_reason_complain_channel].
	pub fn get_action_reason_complain_channel(&self) -> Option<Id<ChannelMarker>> {
		self.action_reason_complain_channel
			.map(|database_id| Id::new(discord_id_from_database_id(database_id)))
	}
}

/// The database representation of a form, a set of default questions that can be given to a user for a particular type
/// of ticket
#[derive(Insertable, Queryable)]
pub struct Form {
	/// The form's ID
	pub id: String,
	/// The database ID of the guild that owns the form.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_guild].
	pub guild: i64,
	/// The name of the form
	pub title: String,
}

impl Form {
	/// Gets the guild that owns the form.
	///
	/// For the raw database representation, use [Self::guild].
	pub fn get_guild(&self) -> Id<GuildMarker> {
		Id::new(discord_id_from_database_id(self.guild))
	}
}

/// The database representation of a question on a form
#[derive(Insertable, Queryable)]
pub struct FormQuestion {
	/// Question's ID
	pub id: String,
	/// The form on which the question is used
	pub form: String,
	/// The position in the form's question order of this question
	pub form_position: i32,
	/// The question text to display
	pub question: String,
}

/// The database representation of a custom ticket category
#[derive(Insertable, Queryable)]
#[diesel(table_name = custom_categories)]
pub struct CustomCategory {
	/// Category's ID
	pub id: String,
	/// The ID of the guild for which the category was created.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_guild].
	pub guild: i64,
	/// The name of the category
	pub name: String,
	/// The ID of the channel to which tickets in this category are posted.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_channel].
	pub channel: i64,
	/// If there's a form associated with the category, this field contains the form ID.
	pub form: Option<String>,
}

impl CustomCategory {
	/// Gets the guild for which the category was created.
	///
	/// For the raw database representation, use [Self::guild].
	pub fn get_build(&self) -> Id<GuildMarker> {
		Id::new(discord_id_from_database_id(self.guild))
	}

	/// Gets the channel to which tickets in this category are posted.
	///
	/// For the raw database representation, use [Self::channel].
	pub fn get_channel(&self) -> Id<ChannelMarker> {
		Id::new(discord_id_from_database_id(self.channel))
	}
}

/// The database representation of a ticket and its conversation metadata
#[derive(Insertable, Queryable)]
pub struct Ticket {
	/// Ticket's ID
	pub id: String,
	/// The ID of the guild the ticket is with.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_guild].
	pub guild: i64,
	/// The ID of the user with whom staff is having the discussion.
	///
	/// To get a Discord-facing version of this more easily, use [Self::get_with_user].
	pub with_user: i64,
	/// The ticket's title
	pub title: String,
	/// If the ticket is using a built-in category, the built-in category
	pub built_in_category: Option<BuiltInTicketCategory>,
	/// If the ticket is using a custom category, the ID of the custom category
	pub custom_category: Option<String>,
}

impl Ticket {
	/// The guild the ticket is with.
	///
	/// For the raw database representation, use [Self::guild].
	pub fn get_guild(&self) -> Id<GuildMarker> {
		Id::new(discord_id_from_database_id(self.guild))
	}

	/// The user with whom staff is having the discussion.
	///
	/// To get a Discord-facing version of this more easily, use [Self::with_user].
	pub fn get_with_user(&self) -> Id<UserMarker> {
		Id::new(discord_id_from_database_id(self.with_user))
	}
}

/// Converts an ID used with Discord (unsigned) to an ID for Postgres use (signed)
pub fn database_id_from_discord_id(discord_id: u64) -> i64 {
	discord_id as i64
}

/// Converts an ID retrieved from the database (signed) to an ID for use with Discord (unsigned)
pub fn discord_id_from_database_id(database_id: i64) -> u64 {
	database_id as u64
}
