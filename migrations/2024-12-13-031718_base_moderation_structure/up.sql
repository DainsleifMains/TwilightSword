-- © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

CREATE DOMAIN discord_id AS BIGINT CONSTRAINT non_zero CHECK (VALUE != 0);

CREATE TABLE guilds (
	guild_id BIGINT PRIMARY KEY,
	start_ticket_channel BIGINT,
	start_ticket_message TEXT NOT NULL,
	start_ticket_interaction BIGINT,
	start_ticket_token TEXT,
	ban_appeal_ticket_channel BIGINT,
	new_partner_ticket_channel BIGINT,
	existing_partner_ticket_channel BIGINT,
	message_reports_channel BIGINT,
	tcn_partner_integration BOOLEAN NOT NULL,
	admin_role BIGINT NOT NULL,
	staff_role BIGINT NOT NULL,
	action_reason_complain_channel BIGINT
);

CREATE TYPE built_in_ticket_category AS ENUM (
	'ban_appeal',
	'new_partner',
	'existing_partner',
	'message_report'
);

CREATE TABLE forms (
	id TEXT PRIMARY KEY,
	guild BIGINT NOT NULL REFERENCES guilds,
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
	guild BIGINT NOT NULL REFERENCES guilds,
	name TEXT NOT NULL,
	channel BIGINT NOT NULL,
	form TEXT REFERENCES forms,
	CONSTRAINT unique_name_for_guild UNIQUE (guild, name)
);

CREATE TABLE tickets (
	id TEXT PRIMARY KEY,
	guild BIGINT NOT NULL REFERENCES guilds,
	with_user BIGINT NOT NULL,
	title TEXT NOT NULL,
	built_in_category built_in_ticket_category,
	custom_category TEXT REFERENCES custom_categories,
	CONSTRAINT has_category CHECK(built_in_category IS NOT NULL OR custom_category IS NOT NULL)
);