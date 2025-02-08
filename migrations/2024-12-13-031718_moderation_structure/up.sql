-- © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

CREATE DOMAIN discord_id AS BIGINT CONSTRAINT non_zero CHECK (VALUE != 0);

CREATE TABLE guilds (
	guild_id discord_id PRIMARY KEY,
	start_ticket_channel discord_id,
	start_ticket_message TEXT NOT NULL,
	start_ticket_message_id discord_id,
	ban_appeal_ticket_channel discord_id,
	new_partner_ticket_channel discord_id,
	existing_partner_ticket_channel discord_id,
	message_reports_channel discord_id,
	tcn_partner_integration BOOLEAN NOT NULL,
	admin_role discord_id NOT NULL,
	staff_role discord_id NOT NULL,
	action_reason_complain_channel discord_id,
	custom_host TEXT UNIQUE
);

CREATE TYPE built_in_ticket_category AS ENUM (
	'ban_appeal',
	'new_partner',
	'existing_partner',
	'message_report'
);

CREATE TABLE forms (
	id TEXT PRIMARY KEY,
	guild discord_id NOT NULL REFERENCES guilds,
	title TEXT NOT NULL
);

ALTER TABLE guilds ADD COLUMN ban_appeal_ticket_form TEXT REFERENCES forms;
ALTER TABLE guilds ADD COLUMN new_partner_ticket_form TEXT REFERENCES forms;
ALTER TABLE guilds ADD COLUMN existing_partner_ticket_form TEXT REFERENCES forms;

CREATE TABLE form_questions (
	id TEXT PRIMARY KEY,
	form TEXT NOT NULL REFERENCES forms,
	form_position INT NOT NULL,
	question TEXT NOT NULL,
	CONSTRAINT questions_ordered_on_form UNIQUE (form, form_position) DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE custom_categories (
	id TEXT PRIMARY KEY,
	guild discord_id NOT NULL REFERENCES guilds,
	name TEXT NOT NULL,
	channel discord_id NOT NULL,
	form TEXT REFERENCES forms,
	CONSTRAINT unique_name_for_guild UNIQUE (guild, name)
);

CREATE TABLE tickets (
	id TEXT PRIMARY KEY,
	guild discord_id NOT NULL REFERENCES guilds,
	with_user discord_id NOT NULL,
	title TEXT NOT NULL,
	built_in_category built_in_ticket_category,
	custom_category TEXT REFERENCES custom_categories,
	is_open BOOLEAN NOT NULL,
	CONSTRAINT has_category CHECK(built_in_category IS NOT NULL OR custom_category IS NOT NULL)
);

CREATE INDEX tickets_for_guild_with_user ON tickets (guild, with_user);

CREATE TABLE ticket_messages (
	id TEXT PRIMARY KEY,
	ticket TEXT NOT NULL REFERENCES tickets,
	author discord_id NOT NULL,
	send_time TIMESTAMP WITH TIME ZONE NOT NULL,
	internal BOOLEAN NOT NULL,
	body TEXT NOT NULL
);

CREATE TYPE automod_action_type AS ENUM (
	'block',
	'disable_communication'
);

CREATE TABLE automod_actions (
	id TEXT PRIMARY KEY,
	guild discord_id NOT NULL REFERENCES guilds,
	target_user discord_id NOT NULL,
	action_type automod_action_type NOT NULL,
	action_time TIMESTAMP WITH TIME ZONE NOT NULL,
	reason TEXT NOT NULL,
	rule_name TEXT NOT NULL,
	message_content TEXT NOT NULL
);

CREATE INDEX automod_target_user_by_guild ON automod_actions (guild, target_user);

CREATE TABLE ban_actions (
	id TEXT PRIMARY KEY,
	guild discord_id NOT NULL REFERENCES guilds,
	banning_user discord_id NOT NULL,
	banned_user discord_id NOT NULL,
	added BOOLEAN NOT NULL,
	action_time TIMESTAMP WITH TIME ZONE NOT NULL,
	reason TEXT NOT NULL
);

CREATE INDEX banned_user_by_guild ON ban_actions (guild, banned_user);

CREATE TABLE kick_actions (
	id TEXT PRIMARY KEY,
	guild discord_id NOT NULL REFERENCES guilds,
	kicking_user discord_id NOT NULL,
	kicked_user discord_id NOT NULL,
	action_time TIMESTAMP WITH TIME ZONE NOT NULL,
	reason TEXT NOT NULL
);

CREATE INDEX kicked_user_by_guild ON kick_actions (guild, kicked_user);

CREATE TABLE timeout_actions (
	id TEXT PRIMARY KEY,
	guild discord_id NOT NULL REFERENCES guilds,
	performing_user discord_id NOT NULL,
	target_user discord_id NOT NULL,
	action_time TIMESTAMP WITH TIME ZONE NOT NULL,
	timeout_until TIMESTAMP WITH TIME ZONE,
	reason TEXT NOT NULL
);

CREATE INDEX timed_out_user_by_guild ON timeout_actions (guild, target_user);

CREATE TABLE pending_partnerships (
	id TEXT PRIMARY KEY,
	guild discord_id NOT NULL REFERENCES guilds,
	partner_guild discord_id NOT NULL,
	invite_code TEXT NOT NULL,
	ticket TEXT NOT NULL REFERENCES tickets
);

CREATE TABLE sessions (
	session_id NUMERIC PRIMARY KEY,
	data TEXT NOT NULL,
	expires TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX session_expiry ON sessions (expires);