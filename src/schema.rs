// @generated automatically by Diesel CLI.

pub mod sql_types {
	#[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "automod_action_type"))]
	pub struct AutomodActionType;

	#[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "built_in_ticket_category"))]
	pub struct BuiltInTicketCategory;
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::AutomodActionType;

	automod_actions (id) {
		id -> Text,
		guild -> Int8,
		target_user -> Int8,
		action_type -> AutomodActionType,
		action_time -> Timestamptz,
		reason -> Text,
		rule_name -> Text,
		message_content -> Text,
	}
}

diesel::table! {
	ban_actions (id) {
		id -> Text,
		guild -> Int8,
		banning_user -> Int8,
		banned_user -> Int8,
		added -> Bool,
		action_time -> Timestamptz,
		reason -> Text,
	}
}

diesel::table! {
	custom_categories (id) {
		id -> Text,
		guild -> Int8,
		name -> Text,
		channel -> Int8,
		form -> Nullable<Text>,
		active -> Bool,
	}
}

diesel::table! {
	form_questions (id) {
		id -> Text,
		form -> Text,
		form_position -> Int4,
		question -> Text,
	}
}

diesel::table! {
	forms (id) {
		id -> Text,
		guild -> Int8,
		title -> Text,
	}
}

diesel::table! {
	guilds (guild_id) {
		guild_id -> Int8,
		start_ticket_channel -> Nullable<Int8>,
		start_ticket_message -> Text,
		start_ticket_message_id -> Nullable<Int8>,
		ban_appeal_ticket_channel -> Nullable<Int8>,
		new_partner_ticket_channel -> Nullable<Int8>,
		existing_partner_ticket_channel -> Nullable<Int8>,
		message_reports_channel -> Nullable<Int8>,
		tcn_partner_integration -> Bool,
		admin_role -> Int8,
		staff_role -> Int8,
		action_reason_complain_channel -> Nullable<Int8>,
		custom_host -> Nullable<Text>,
		ban_appeal_ticket_form -> Nullable<Text>,
		new_partner_ticket_form -> Nullable<Text>,
		existing_partner_ticket_form -> Nullable<Text>,
	}
}

diesel::table! {
	kick_actions (id) {
		id -> Text,
		guild -> Int8,
		kicking_user -> Int8,
		kicked_user -> Int8,
		action_time -> Timestamptz,
		reason -> Text,
	}
}

diesel::table! {
	pending_partnerships (id) {
		id -> Text,
		guild -> Int8,
		partner_guild -> Int8,
		invite_code -> Text,
		ticket -> Text,
	}
}

diesel::table! {
	sessions (session_id) {
		session_id -> Numeric,
		data -> Text,
		expires -> Timestamptz,
	}
}

diesel::table! {
	ticket_messages (id) {
		id -> Text,
		ticket -> Text,
		author -> Int8,
		send_time -> Timestamptz,
		body -> Text,
		staff_message -> Int8,
		user_message -> Nullable<Int8>,
	}
}

diesel::table! {
	ticket_restricted_users (guild_id, user_id) {
		guild_id -> Int8,
		user_id -> Int8,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::BuiltInTicketCategory;

	tickets (id) {
		id -> Text,
		guild -> Int8,
		with_user -> Int8,
		title -> Text,
		built_in_category -> Nullable<BuiltInTicketCategory>,
		custom_category -> Nullable<Text>,
		staff_thread -> Int8,
		user_thread -> Int8,
		closed_at -> Nullable<Timestamptz>,
	}
}

diesel::table! {
	timeout_actions (id) {
		id -> Text,
		guild -> Int8,
		performing_user -> Int8,
		target_user -> Int8,
		action_time -> Timestamptz,
		timeout_until -> Nullable<Timestamptz>,
		reason -> Text,
	}
}

diesel::joinable!(automod_actions -> guilds (guild));
diesel::joinable!(ban_actions -> guilds (guild));
diesel::joinable!(custom_categories -> forms (form));
diesel::joinable!(custom_categories -> guilds (guild));
diesel::joinable!(form_questions -> forms (form));
diesel::joinable!(kick_actions -> guilds (guild));
diesel::joinable!(pending_partnerships -> guilds (guild));
diesel::joinable!(pending_partnerships -> tickets (ticket));
diesel::joinable!(ticket_messages -> tickets (ticket));
diesel::joinable!(tickets -> custom_categories (custom_category));
diesel::joinable!(tickets -> guilds (guild));
diesel::joinable!(timeout_actions -> guilds (guild));

diesel::allow_tables_to_appear_in_same_query!(
	automod_actions,
	ban_actions,
	custom_categories,
	form_questions,
	forms,
	guilds,
	kick_actions,
	pending_partnerships,
	sessions,
	ticket_messages,
	ticket_restricted_users,
	tickets,
	timeout_actions,
);
