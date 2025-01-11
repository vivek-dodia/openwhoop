use std::collections::BTreeSet;

use btleplug::{
    api::{CharPropFlags, Characteristic, Peripheral as _, WriteType},
    platform::Peripheral,
};
use futures::StreamExt;
use uuid::Uuid;
use whoop::{
    constants::{MetadataType, CMD_TO_STRAP, DATA_FROM_STRAP, WHOOP_SERVICE},
    WhoopData, WhoopPacket,
};

use crate::DatabaseHandler;

pub struct Whoop {
    peripheral: Peripheral,
    db: DatabaseHandler,
}

impl Whoop {
    pub fn new(peripheral: Peripheral, db: DatabaseHandler) -> Self {
        Self { peripheral, db }
    }

    pub async fn connect(&mut self) -> anyhow::Result<()> {
        self.peripheral.connect().await?;
        self.peripheral.discover_services().await?;
        Ok(())
    }

    fn create_char(characteristic: Uuid) -> Characteristic {
        Characteristic {
            uuid: characteristic,
            service_uuid: WHOOP_SERVICE,
            properties: CharPropFlags::empty(),
            descriptors: BTreeSet::new(),
        }
    }
    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        self.peripheral
            .subscribe(&Self::create_char(DATA_FROM_STRAP))
            .await?;

        Ok(())
    }

    pub async fn send_command(&mut self, packet: WhoopPacket) -> anyhow::Result<()> {
        self.peripheral
            .write(
                &Self::create_char(CMD_TO_STRAP),
                &packet.framed_packet(),
                WriteType::WithoutResponse,
            )
            .await?;
        Ok(())
    }

    pub async fn sync_history(&mut self) -> anyhow::Result<()> {
        let mut notifications = self.peripheral.notifications().await?;

        let start_sync = WhoopPacket::history_start();
        self.send_command(start_sync).await?;

        while let Some(notification) = notifications.next().await {
            let packet = self
                .db
                .create_packet(notification.uuid, notification.value)
                .await?;

            match packet.uuid {
                DATA_FROM_STRAP => {
                    let packet = WhoopPacket::from_data(packet.bytes)?;

                    let Ok(data) = WhoopData::from_packet(packet) else {
                        continue;
                    };

                    match data {
                        WhoopData::HistoryReading { unix, bpm, rr } => {
                            self.db.create_reading(unix, bpm, rr).await?;
                        }
                        WhoopData::HistoryMetadata { data, cmd, .. } => match cmd {
                            MetadataType::HistoryComplete => break,
                            MetadataType::HistoryStart => {}
                            MetadataType::HistoryEnd => {
                                let packet = WhoopPacket::history_end(data);
                                self.send_command(packet).await?;
                            }
                        },
                        WhoopData::ConsoleLog { .. } => {}
                        WhoopData::RunAlarm { .. } => {}
                        WhoopData::Event { .. } => {}
                        WhoopData::UnknownEvent { .. } => {}
                    }
                }
                _ => {
                    // todo!()
                }
            }
        }
        Ok(())
    }
}
