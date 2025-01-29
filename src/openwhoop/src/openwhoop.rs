use btleplug::api::ValueNotification;
use db_entities::packets;
use whoop::{
    constants::{MetadataType, DATA_FROM_STRAP},
    HistoryReading, WhoopData, WhoopPacket,
};

use crate::DatabaseHandler;

pub struct OpenWhoop {
    pub database: DatabaseHandler,
}

impl OpenWhoop {
    pub fn new(database: DatabaseHandler) -> Self {
        Self { database }
    }

    pub async fn store_packet(
        &self,
        notification: ValueNotification,
    ) -> anyhow::Result<packets::Model> {
        let packet = self
            .database
            .create_packet(notification.uuid, notification.value)
            .await?;

        Ok(packet)
    }

    pub async fn handle_packet(
        &self,
        packet: packets::Model,
    ) -> anyhow::Result<Option<WhoopPacket>> {
        match packet.uuid {
            DATA_FROM_STRAP => {
                let packet = WhoopPacket::from_data(packet.bytes)?;

                let Ok(data) = WhoopData::from_packet(packet) else {
                    return Ok(None);
                };

                match data {
                    WhoopData::HistoryReading(HistoryReading {
                        unix,
                        bpm,
                        rr,
                        activity,
                    }) => {
                        self.database
                            .create_reading(unix, bpm, rr, activity as i64)
                            .await?;
                    }
                    WhoopData::HistoryMetadata { data, cmd, .. } => match cmd {
                        MetadataType::HistoryComplete => return Ok(None),
                        MetadataType::HistoryStart => {}
                        MetadataType::HistoryEnd => {
                            let packet = WhoopPacket::history_end(data);
                            return Ok(Some(packet));
                        }
                    },
                    WhoopData::ConsoleLog { log, .. } => {
                        trace!(target: "ConsoleLog", "{}", log);
                    }
                    WhoopData::RunAlarm { .. } => {}
                    WhoopData::Event { .. } => {}
                    WhoopData::UnknownEvent { .. } => {}
                }
            }
            _ => {
                // todo!()
            }
        }

        Ok(None)
    }
}
