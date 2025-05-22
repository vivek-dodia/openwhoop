use std::{collections::BTreeSet, time::Duration};

use btleplug::{
    api::{CharPropFlags, Characteristic, Peripheral as _, WriteType},
    platform::Peripheral,
};
use db_entities::packets::Model;
use futures::StreamExt;
use tokio::time::sleep;
use uuid::Uuid;
use whoop::{
    constants::{
        CMD_FROM_STRAP, CMD_TO_STRAP, DATA_FROM_STRAP, EVENTS_FROM_STRAP, MEMFAULT, WHOOP_SERVICE,
    },
    WhoopPacket,
};

use crate::{db::DatabaseHandler, openwhoop::OpenWhoop};

pub struct WhoopDevice {
    peripheral: Peripheral,
    whoop: OpenWhoop,
    debug_packets: bool,
}

impl WhoopDevice {
    pub fn new(peripheral: Peripheral, db: DatabaseHandler, debug_packets: bool) -> Self {
        Self {
            peripheral,
            whoop: OpenWhoop::new(db),
            debug_packets,
        }
    }

    pub async fn connect(&mut self) -> anyhow::Result<()> {
        self.peripheral.connect().await?;
        self.peripheral.discover_services().await?;
        Ok(())
    }

    pub async fn is_connected(&mut self) -> anyhow::Result<bool> {
        let is_connected = self.peripheral.is_connected().await?;
        Ok(is_connected)
    }

    fn create_char(characteristic: Uuid) -> Characteristic {
        Characteristic {
            uuid: characteristic,
            service_uuid: WHOOP_SERVICE,
            properties: CharPropFlags::empty(),
            descriptors: BTreeSet::new(),
        }
    }

    async fn subscribe(&self, char: Uuid) -> anyhow::Result<()> {
        self.peripheral.subscribe(&Self::create_char(char)).await?;
        Ok(())
    }

    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        self.subscribe(DATA_FROM_STRAP).await?;
        self.subscribe(CMD_FROM_STRAP).await?;
        self.subscribe(EVENTS_FROM_STRAP).await?;
        self.subscribe(MEMFAULT).await?;

        self.send_command(WhoopPacket::enter_high_freq_sync())
            .await?;

        // self.send_command(WhoopPacket::hello_harvard()).await?;
        // self.send_command(WhoopPacket::set_time()).await?;
        // self.send_command(WhoopPacket::get_name()).await?;

        Ok(())
    }

    pub async fn send_command(&mut self, packet: WhoopPacket) -> anyhow::Result<()> {
        let packet = packet.framed_packet();
        self.peripheral
            .write(
                &Self::create_char(CMD_TO_STRAP),
                &packet,
                WriteType::WithoutResponse,
            )
            .await?;
        Ok(())
    }

    pub async fn sync_history(&mut self) -> anyhow::Result<()> {
        let mut notifications = self.peripheral.notifications().await?;
        // self.send_command(WhoopPacket::toggle_r7_data_collection())
        //     .await?;
        self.send_command(WhoopPacket::history_start()).await?;

        'a: loop {
            let notification = notifications.next();
            let sleep_ = sleep(Duration::from_secs(10));

            tokio::select! {
                _ = sleep_ => {
                    if self.on_sleep().await? {
                        error!("Whoop disconnected");
                        for _ in 0..5{
                            if self.connect().await.is_ok() {
                                self.initialize().await?;
                                self.send_command(WhoopPacket::history_start()).await?;
                                continue 'a;
                            }

                            sleep(Duration::from_secs(10)).await;
                        }

                        break;
                    }
                },
                Some(notification) = notification => {
                    let packet = match self.debug_packets {
                        true => self.whoop.store_packet(notification).await?,
                        false => Model { id: 0, uuid: notification.uuid, bytes: notification.value },
                    };

                    if let Some(packet) = self.whoop.handle_packet(packet).await?{
                        self.send_command(packet).await?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn on_sleep(&mut self) -> anyhow::Result<bool> {
        let is_connected = self.peripheral.is_connected().await?;
        Ok(!is_connected)
    }
}
