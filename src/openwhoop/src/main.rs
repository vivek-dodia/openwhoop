use std::time::Duration;

use anyhow::anyhow;
use btleplug::{
    api::{BDAddr, Central, Manager as _, Peripheral as _, ScanFilter},
    platform::{Adapter, Manager, Peripheral},
};
use clap::{Parser, Subcommand};
use dotenv::dotenv;
use openwhoop::{DatabaseHandler, Whoop};
use tokio::time::sleep;
use whoop::constants::WHOOP_SERVICE;

#[derive(Parser)]
pub struct OpenWhoop {
    #[arg(env, long)]
    pub database_url: String,
    #[arg(env, long)]
    pub ble_interface: Option<String>,
    #[clap(subcommand)]
    pub subcommand: OpenWhoopCommand,
}

#[derive(Subcommand)]
pub enum OpenWhoopCommand {
    Scan,
    DownloadHistory {
        #[arg(long, env)]
        whoop_addr: BDAddr,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(error) = dotenv() {
        println!("{}", error);
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("sqlx::query", log::LevelFilter::Off)
        .init();

    let cli = OpenWhoop::parse();
    let db_handler = DatabaseHandler::new(cli.database_url).await;

    let manager = Manager::new().await?;
    let adapter = match cli.ble_interface {
        Some(interface) => {
            let adapters = manager.adapters().await?;
            let mut c_adapter = Err(anyhow!("Adapter: `{}` not found", interface));
            for adapter in adapters {
                let name = adapter.adapter_info().await?;
                if name == interface {
                    c_adapter = Ok(adapter);
                    break;
                }
            }

            c_adapter?
        }
        None => {
            let adapters = manager.adapters().await?;
            adapters
                .into_iter()
                .next()
                .ok_or(anyhow!("No BLE adapters found"))?
        }
    };

    match cli.subcommand {
        OpenWhoopCommand::Scan => {
            scan_command(adapter, None).await?;
            Ok(())
        }
        OpenWhoopCommand::DownloadHistory { whoop_addr } => {
            let peripheral = scan_command(adapter, Some(whoop_addr)).await?;
            let mut whoop = Whoop::new(peripheral, db_handler);

            whoop.connect().await?;
            whoop.initialize().await?;

            whoop.sync_history().await
        }
    }
}

async fn scan_command(
    adapter: Adapter,
    peripheral_addr: Option<BDAddr>,
) -> anyhow::Result<Peripheral> {
    adapter
        .start_scan(ScanFilter {
            services: vec![WHOOP_SERVICE],
        })
        .await?;

    loop {
        let peripherals = adapter.peripherals().await?;

        for peripheral in peripherals {
            let Some(properties) = peripheral.properties().await? else {
                continue;
            };

            if !properties.services.contains(&WHOOP_SERVICE) {
                continue;
            }

            let Some(peripheral_addr) = peripheral_addr else {
                println!("Address: {}", properties.address);
                println!("Name: {:?}", properties.local_name);
                println!("RSSI: {:?}", properties.rssi);
                println!();
                continue;
            };

            if properties.address == peripheral_addr {
                return Ok(peripheral);
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
}
