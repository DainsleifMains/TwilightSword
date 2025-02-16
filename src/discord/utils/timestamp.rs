// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use chrono::{DateTime, TimeZone, Utc};
use twilight_model::util::datetime::{Timestamp, TimestampParseError};
use twilight_util::snowflake::Snowflake;

/// Gets the timestamp from the ID snowflake. If any failures occur in the conversion, returns `None`.
pub fn datetime_from_id(id: impl Snowflake) -> Option<DateTime<Utc>> {
	let timestamp = id.timestamp();
	Utc.timestamp_millis_opt(timestamp).single()
}

/// Gets the [DateTime] object for a timestamp from Discord. If any failures occur in the conversion, returns `None`.
pub fn datetime_from_timestamp(timestamp: &Timestamp) -> Option<DateTime<Utc>> {
	let micros = timestamp.as_micros();
	Utc.timestamp_micros(micros).single()
}

/// Gets a [Timestamp] object from the ID snowflake.
pub fn timestamp_from_id(id: impl Snowflake) -> Result<Timestamp, TimestampParseError> {
	Timestamp::from_micros(id.timestamp() * 1000)
}
