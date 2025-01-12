use chrono::Utc;

use crate::{
    constants::{CommandNumber, PacketType},
    WhoopPacket,
};

impl WhoopPacket {
    pub fn history_start() -> WhoopPacket {
        WhoopPacket::new(
            PacketType::Command,
            0,
            CommandNumber::SendHistoricalData.as_u8(),
            vec![0x00],
        )
    }

    pub fn hello_harvard() -> WhoopPacket {
        WhoopPacket::new(
            PacketType::Command,
            0,
            CommandNumber::GetHelloHarvard.as_u8(),
            vec![0x00],
        )
    }

    pub fn get_name() -> WhoopPacket {
        WhoopPacket::new(
            PacketType::Command,
            0,
            CommandNumber::GetAdvertisingNameHarvard.as_u8(),
            vec![0x00],
        )
    }

    pub fn set_time() -> WhoopPacket {
        let mut data = vec![];
        let current_time = Utc::now().timestamp() as u32;
        data.extend_from_slice(&current_time.to_le_bytes());
        data.append(&mut vec![0, 0, 0, 0, 0]); // padding
        WhoopPacket::new(
            PacketType::Command,
            0,
            CommandNumber::SetClock.as_u8(),
            data,
        )
    }

    pub fn history_end(data: u32) -> WhoopPacket {
        let mut packet_data = vec![0x01];
        packet_data.extend_from_slice(&data.to_le_bytes());
        packet_data.append(&mut vec![0, 0, 0, 0]); // padding

        WhoopPacket::new(
            PacketType::Command,
            0,
            CommandNumber::HistoricalDataResult.as_u8(),
            packet_data,
        )
    }
}
