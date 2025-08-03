const HR_SERVICE_ID: u16 = 0x000b;
const HR_ATTR_ID: u16 = 0x000c;

#[path = "../hr_data.rs"]
mod hr_data;

use anyhow::bail;
use bluer::{AdapterEvent, Address, Device};
use clap::{Parser, ValueEnum};
use futures::{StreamExt, pin_mut};
use hr_data::HRData;
use log::{debug, error, info};
use std::{str::FromStr, time::Duration};
use tokio::time::timeout;

async fn hr_loop(device: Device, format: PrintFormat) -> bluer::Result<()> {
    // seems to be the only way to wait for service resolve
    let _ = device.services().await?; 
    let service = device.service(HR_SERVICE_ID).await?;
    let characteristic = service.characteristic(HR_ATTR_ID).await?;
    let notify = characteristic.notify().await?;
    pin_mut!(notify);

    info!("Starting notification loop...");
    loop {
        match notify.next().await {
            Some(value) => match HRData::try_from(value) {
                Ok(data) => {
                    let s = match format {
                        PrintFormat::Print => format!("{data:?}"),
                        PrintFormat::Json => {
                            serde_json::to_string(&data).unwrap()
                        }
                    };
                    println!("{s}");
                }
                Err(e) => error!("Could not parse HR data:\n {e}"),
            },
            None => {
                info!("Notification session was terminated");
                break;
            }
        }
    }

    Ok(())
}

async fn find_device(to_find: Address) -> bluer::Result<Device> {
    info!("Trying to find device with address {to_find}...");
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let discover = adapter.discover_devices().await?;
    pin_mut!(discover);
    while let Some(evt) = discover.next().await {
        if let AdapterEvent::DeviceAdded(addr) = evt {
            debug!("Discovered device: {addr}");
            if addr == to_find {
                return adapter.device(addr);
            }
        }
    }

    Err(bluer::Error {
        kind: bluer::ErrorKind::NotAvailable,
        message: "Requested device not available".to_owned(),
    })
}

#[derive(ValueEnum, Debug, Clone)]
enum PrintFormat {
    Print,
    Json,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CliArgs {
    device: String,
    /// The output format for the gathered heart rate data (stdout)
    #[arg(value_enum, default_value_t = PrintFormat::Print, short = 'f')]
    format: PrintFormat,
    /// Amount of seconds before giving up on the BT connect process
    #[arg(default_value_t = 30, short = 't')]
    connect_timeout: u64,
}

async fn run() -> anyhow::Result<()> {
    let args = CliArgs::parse();
    debug!("Running with configuration: {args:?}");

    let addr = match Address::from_str(&args.device) {
        Ok(addr) => addr,
        Err(e) => bail!("Faulty address given: {e}"),
    };

    let dur = Duration::from_secs(args.connect_timeout);
    let device = match timeout(dur, find_device(addr)).await {
        Ok(device) => device?,
        Err(_) => {
            bail!("Could not connect to {} within {:?}", addr, dur);
        }
    };

    device.connect().await?;
    hr_loop(device, args.format).await?;
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

     if let Err(e) = run().await {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
