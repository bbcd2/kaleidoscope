use crate::filters::ServerError;

use anyhow::{anyhow, Context as _, Result};
use chrono::NaiveDateTime;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, Pool, PooledConnection},
};
use serde::Serialize;
use warp::{reject, Filter};

pub type PoolPg = Pool<ConnectionManager<PgConnection>>;
pub type PooledPg = PooledConnection<ConnectionManager<PgConnection>>;

pub type UserId = i32;
pub type Channel = i32;

/// Establish a pool and database connection from `DATABASE_URL`
pub fn establish_connection() -> Result<PoolPg> {
    let database_url =
        std::env::var("DATABASE_URL").expect("not set environment variable: DATABASE_URL");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = PoolPg::new(manager).context("failed to create pool")?;
    Ok(pool)
}

pub struct Database {
    pub connection: PooledPg,
}
impl Database {
    pub fn create_recording(&mut self, recording: &RecordingUpdate) -> Result<Recording> {
        let recording = diesel::insert_into(crate::schema::recordings::table)
            .values(recording)
            .get_result(&mut self.connection)
            .context("failed to insert recording")?;
        Ok(recording)
    }
    pub fn delete_recording(&mut self, target_id: i32) -> Result<()> {
        use crate::schema::recordings::dsl::*;
        diesel::delete(recordings.filter(id.eq(target_id))).execute(&mut self.connection)?;
        Ok(())
    }
    pub fn get_recordings(&mut self, start: i64, count: i64) -> Result<Vec<Recording>> {
        use crate::schema::recordings::dsl::*;
        let recordings_list = recordings
            .offset(start)
            .limit(count)
            .load(&mut self.connection)?;
        Ok(recordings_list)
    }
    pub fn update_recording(&mut self, recording: &RecordingUpdate) -> Result<Recording> {
        use crate::schema::recordings::dsl::*;
        let recording = diesel::update(recordings.filter(uuid.eq(recording.uuid)))
            .set(recording)
            .get_result(&mut self.connection)
            .context("failed to update recording row")?;
        Ok(recording)
    }
}

#[derive(Queryable, Selectable, Serialize, Clone)]
#[diesel(table_name = crate::schema::recordings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Recording {
    pub id: i32,
    pub user_id: Option<UserId>,
    pub uuid: String,
    pub rec_start: NaiveDateTime,
    pub rec_end: NaiveDateTime,
    pub status: String,
    pub stage: i32,
    pub channel: Channel,
}
#[derive(Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::recordings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RecordingUpdate<'a> {
    pub user_id: Option<UserId>,
    pub uuid: &'a str,
    pub rec_start: &'a NaiveDateTime,
    pub rec_end: &'a NaiveDateTime,
    pub status: String,
    pub stage: i32,
    pub channel: Channel,
}

// todo: users

/// Filter for accessing the database
pub fn with_database(
    pool: PoolPg,
) -> impl Filter<Extract = (Database,), Error = warp::Rejection> + Clone {
    warp::any()
        .map(move || pool.clone())
        .and_then(|pool: PoolPg| async move {
            match pool.get() {
                Ok(pool) => Ok(Database { connection: pool }),
                Err(e) => Err(reject::custom(ServerError::new(anyhow!(
                    "failed to access database: {e}"
                )))),
            }
        })
}
