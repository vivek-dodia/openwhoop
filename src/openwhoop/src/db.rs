use chrono::{Local, NaiveDateTime, TimeZone};
use migration::{Migrator, MigratorTrait};
use sea_orm::{ActiveModelTrait, ActiveValue::NotSet, Database, DatabaseConnection, Set};
use uuid::Uuid;

pub struct DatabaseHandler {
    db: DatabaseConnection,
}

impl DatabaseHandler {
    pub async fn new(path: String) -> Self {
        let db = Database::connect(path)
            .await
            .expect("Unable to connect to db");

        Migrator::up(&db, None)
            .await
            .expect("Error running migrations");

        Self { db }
    }

    pub async fn create_packet(
        &self,
        char: Uuid,
        data: Vec<u8>,
    ) -> anyhow::Result<db_entities::packets::Model> {
        let packet = db_entities::packets::ActiveModel {
            id: NotSet,
            uuid: Set(char),
            bytes: Set(data),
        };

        let packet = packet.insert(&self.db).await?;
        Ok(packet)
    }

    pub async fn create_reading(
        &self,
        unix: u32,
        bpm: u8,
        rr: Vec<u16>,
    ) -> anyhow::Result<db_entities::heart_rate::Model> {
        let packet = db_entities::heart_rate::ActiveModel {
            id: NotSet,
            bpm: Set(bpm as i16),
            time: Set(timestamp_to_local(unix)),
            rr_intervals: Set(rr_to_string(rr)),
        };

        let model = packet.insert(&self.db).await?;
        Ok(model)
    }
}

fn timestamp_to_local(unix: u32) -> NaiveDateTime {
    let dt = Local
        .timestamp_opt(unix as i64, 0)
        .single()
        .expect("I don't know");

    dt.naive_local()
}

fn rr_to_string(rr: Vec<u16>) -> String {
    rr.iter().map(u16::to_string).collect::<Vec<_>>().join(",")
}
