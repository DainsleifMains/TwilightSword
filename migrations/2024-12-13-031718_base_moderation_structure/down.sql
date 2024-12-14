-- © 2024 ElementalAlchemist and the Dainsleif Mains Development Team
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE guilds DROP COLUMN ban_appeal_ticket_form;
ALTER TABLE guilds DROP COLUMN new_partner_ticket_form;
ALTER TABLE guilds DROP COLUMN existing_partner_ticket_form;

DROP TABLE tickets;
DROP TABLE custom_categories;
DROP TABLE form_questions;
DROP TABLE forms;
DROP TYPE built_in_ticket_category;
DROP TABLE guilds;