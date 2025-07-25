const HR_SERVICE_ID: u16 = 0x000b;
const HR_ATTR_ID: u16 = 0x000c;

#[path = "../hr_data.rs"]
mod hr_data;

use bluer::{AdapterEvent, Address, Device};
use clap::{Parser, ValueEnum};
use futures::{StreamExt, pin_mut};
use hr_data::HRData;
use log::{debug, error, info};
use std::str::FromStr;

async fn hr_loop(device: Device, format: PrintFormat) -> bluer::Result<()> {
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
                        PrintFormat::Json => serde_json::to_string(&data).unwrap(),
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
    #[arg(value_enum, default_value_t = PrintFormat::Print, short = 'f')]
    format: PrintFormat,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    env_logger::init();
    let args = CliArgs::parse();
    debug!("Running with configuration: {args:?}");

    let addr_to_find = Address::from_str(&args.device).unwrap();
    let device = find_device(addr_to_find).await?;
    device.connect().await?;
    // TODO: this happens to fast right now, need to await something probably?
    hr_loop(device, args.format).await
}
