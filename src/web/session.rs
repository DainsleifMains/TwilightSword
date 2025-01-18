// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::model::Session;
use crate::schema::sessions;
use async_trait::async_trait;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use time::OffsetDateTime;
use tower_sessions::session::{Id, Record};
use tower_sessions::{session_store, SessionStore};

#[derive(Clone, Debug)]
pub struct DatabaseStore {
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
}

impl DatabaseStore {
	pub fn new(db_connection_pool: Pool<ConnectionManager<PgConnection>>) -> Self {
		Self { db_connection_pool }
	}
}

#[async_trait]
impl SessionStore for DatabaseStore {
	async fn create(&self, record: &mut Record) -> session_store::Result<()> {
		let mut db_connection = match self.db_connection_pool.get() {
			Ok(connection) => connection,
			Err(error) => {
				return Err(session_store::Error::Backend(format!(
					"Couldn't get database connection: {}",
					error
				)))
			}
		};

		let session_id: BigDecimal = record.id.0.into();

		let data = match serde_json::to_string(&record.data) {
			Ok(data) => data,
			Err(error) => return Err(session_store::Error::Encode(format!("{:?}", error))),
		};

		let expires = record.expiry_date.unix_timestamp_nanos();
		let expires = match expires.try_into() {
			Ok(expiry) => expiry,
			Err(error) => {
				return Err(session_store::Error::Backend(format!(
					"Timestamp out of bounds: {}",
					error
				)))
			}
		};
		let expires = DateTime::from_timestamp_nanos(expires);

		let new_session = Session {
			session_id,
			data,
			expires,
		};

		let db_result = diesel::insert_into(sessions::table)
			.values(new_session)
			.execute(&mut db_connection);
		if let Err(error) = db_result {
			return Err(session_store::Error::Backend(format!(
				"Failed to create new session: {}",
				error
			)));
		}

		Ok(())
	}

	async fn save(&self, record: &Record) -> session_store::Result<()> {
		let mut db_connection = match self.db_connection_pool.get() {
			Ok(connection) => connection,
			Err(error) => {
				return Err(session_store::Error::Backend(format!(
					"Couldn't get database connection: {}",
					error
				)))
			}
		};

		let session_id: BigDecimal = record.id.0.into();

		let data = match serde_json::to_string(&record.data) {
			Ok(data) => data,
			Err(error) => return Err(session_store::Error::Encode(format!("{:?}", error))),
		};

		let expires = record.expiry_date.unix_timestamp_nanos();
		let expires = match expires.try_into() {
			Ok(expiry) => expiry,
			Err(error) => {
				return Err(session_store::Error::Backend(format!(
					"Timestamp out of bounds: {}",
					error
				)))
			}
		};
		let expires = DateTime::from_timestamp_nanos(expires);

		let db_result = diesel::update(sessions::table)
			.filter(sessions::session_id.eq(session_id))
			.set((sessions::data.eq(data), sessions::expires.eq(expires)))
			.execute(&mut db_connection);
		if let Err(error) = db_result {
			return Err(session_store::Error::Backend(format!(
				"Failed to update session: {}",
				error
			)));
		}

		Ok(())
	}

	async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
		let mut db_connection = match self.db_connection_pool.get() {
			Ok(connection) => connection,
			Err(error) => {
				return Err(session_store::Error::Backend(format!(
					"Couldn't get database connection: {}",
					error
				)))
			}
		};

		let current_datetime = Utc::now();
		let expire_result = diesel::delete(sessions::table)
			.filter(sessions::expires.le(current_datetime))
			.execute(&mut db_connection);
		if let Err(error) = expire_result {
			return Err(session_store::Error::Backend(format!(
				"Failed to expire old sessions: {}",
				error
			)));
		}

		let db_session_id: BigDecimal = session_id.0.into();
		let db_result: QueryResult<Option<Session>> =
			sessions::table.find(db_session_id).first(&mut db_connection).optional();

		match db_result {
			Ok(Some(session)) => {
				let data = match serde_json::from_str(&session.data) {
					Ok(data) => data,
					Err(error) => return Err(session_store::Error::Decode(format!("{:?}", error))),
				};
				let expiry_date = session.expires.timestamp_nanos_opt();
				let expiry_date = match expiry_date {
					Some(ts) => ts.into(),
					None => {
						return Err(session_store::Error::Backend(String::from(
							"Out of range expiration timestamp",
						)))
					}
				};
				let expiry_date = match OffsetDateTime::from_unix_timestamp_nanos(expiry_date) {
					Ok(time) => time,
					Err(error) => {
						return Err(session_store::Error::Backend(format!(
							"Timestamp conversion error: {}",
							error
						)))
					}
				};

				Ok(Some(Record {
					id: *session_id,
					data,
					expiry_date,
				}))
			}
			Ok(None) => Ok(None),
			Err(error) => Err(session_store::Error::Backend(format!(
				"Couldn't retrieve session from database: {}",
				error
			))),
		}
	}

	async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
		let mut db_connection = match self.db_connection_pool.get() {
			Ok(connection) => connection,
			Err(error) => {
				return Err(session_store::Error::Backend(format!(
					"Couldn't get database connection: {}",
					error
				)))
			}
		};

		let db_session_id: BigDecimal = session_id.0.into();
		let db_result = diesel::delete(sessions::table)
			.filter(sessions::session_id.eq(db_session_id))
			.execute(&mut db_connection);

		match db_result {
			Ok(_) => Ok(()),
			Err(error) => Err(session_store::Error::Backend(format!(
				"Failed to delete session: {}",
				error
			))),
		}
	}
}
