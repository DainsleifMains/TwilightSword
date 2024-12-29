// @generated automatically by Diesel CLI.

pub mod sql_types {
	#[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "built_in_ticket_category"))]
	pub struct BuiltInTicketCategory;
}

diesel::table! {
	custom_categories (id) {
		id -> Text,
		guild -> Int8,
		name -> Text,
		channel -> Int8,
		form -> Nullable<Text>,
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
		ban_appeal_ticket_form -> Nullable<Text>,
		new_partner_ticket_form -> Nullable<Text>,
		existing_partner_ticket_form -> Nullable<Text>,
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
	}
}

diesel::joinable!(custom_categories -> forms (form));
diesel::joinable!(custom_categories -> guilds (guild));
diesel::joinable!(form_questions -> forms (form));
diesel::joinable!(tickets -> custom_categories (custom_category));
diesel::joinable!(tickets -> guilds (guild));

diesel::allow_tables_to_appear_in_same_query!(custom_categories, form_questions, forms, guilds, tickets,);
