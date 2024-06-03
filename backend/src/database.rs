use std::time::SystemTime;

use anyhow::{Context as _, Result};
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, Pool, PooledConnection},
};
use serde::Serialize;

pub type PoolPg = Pool<ConnectionManager<PgConnection>>;
pub type PooledPg = PooledConnection<ConnectionManager<PgConnection>>;

pub type UserId = i32;
pub type Channel = i32;

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
    pub fn create_video(&mut self, video: NewVideo) -> Result<Video> {
        let video = diesel::insert_into(crate::schema::videos::table)
            .values(&video)
            .get_result(&mut self.connection)
            .context("failed to insert video")?;
        Ok(video)
    }
    pub fn delete_video(&mut self, target_id: i32) -> Result<()> {
        use crate::schema::videos::dsl::*;
        diesel::delete(videos.filter(id.eq(target_id))).execute(&mut self.connection)?;
        Ok(())
    }
    pub fn get_videos(&mut self) -> Result<Vec<Video>> {
        use crate::schema::videos::dsl::*;
        let videos_list = videos.load(&mut self.connection)?;
        Ok(videos_list)
    }
}

#[derive(Queryable, Selectable, Serialize, Clone)]
#[diesel(table_name = crate::schema::videos)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Video {
    pub id: i32,
    pub user_id: Option<UserId>,
    pub uuid: String,
    pub rec_start: SystemTime,
    pub rec_end: SystemTime,
    pub status: String,
    pub channel: Channel,
}
#[derive(Insertable)]
#[diesel(table_name = crate::schema::videos)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewVideo<'a> {
    pub user_id: Option<UserId>,
    pub uuid: &'a str,
    pub rec_start: &'a SystemTime,
    pub rec_end: &'a SystemTime,
    pub status: String,
    pub channel: Channel,
}

// todo: users
