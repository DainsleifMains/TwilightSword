-- © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE guilds DROP COLUMN ban_appeal_ticket_form;
ALTER TABLE guilds DROP COLUMN new_partner_ticket_form;
ALTER TABLE guilds DROP COLUMN existing_partner_ticket_form;

DROP INDEX session_expiry;
DROP TABLE sessions;
DROP INDEX timed_out_user_by_guild;
DROP INDEX kicked_user_by_guild;
DROP INDEX banned_user_by_guild;
DROP INDEX automod_target_user_by_guild;
DROP TABLE timeout_actions;
DROP TABLE kick_actions;
DROP TABLE ban_actions;
DROP TABLE automod_actions;
DROP TYPE automod_action_type;
DROP INDEX tickets_for_guild_with_user;
DROP TABLE tickets;
DROP TABLE custom_categories;
DROP TABLE form_questions;
DROP TABLE forms;
DROP TYPE built_in_ticket_category;
DROP TABLE guilds;
DROP DOMAIN discord_id;