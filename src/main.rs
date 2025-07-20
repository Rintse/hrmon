const HR_SERVICE_ID: u16 = 0x000b;
const HR_ATTR_ID: u16 = 0x000c;

mod hr_data;

use bluer::{AdapterEvent, Address, Device};
use clap::{Arg, Command};
use futures::{StreamExt, pin_mut};
use hr_data::HRData2;
use log::debug;
use std::str::FromStr;

async fn hr_loop(device: Device) -> bluer::Result<()> {
    let service = device.service(HR_SERVICE_ID).await?;
    let characteristic = service.characteristic(HR_ATTR_ID).await?;
    let notify = characteristic.notify().await?;
    pin_mut!(notify);

    loop {
        match notify.next().await {
            Some(value) => match HRData2::try_from(value) {
                Ok(data) => match data {
                    HRData2::Single(hrdata) => {
                        println!("Single-byte precision HR data: {:?}", hrdata)
                    }
                    HRData2::Double(hrdata) => {
                        println!("Double-byte precision HR data: {:?}", hrdata)
                    }
                },
                Err(e) => println!("Could not parse HR data:\n {}", e),
            },
            None => {
                println!("Notification session was terminated");
                break;
            }
        }
    }

    Ok(())
}

async fn find_device(to_find: Address) -> bluer::Result<Device> {
    debug!("Trying to find device with address {to_find}...");
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let discover = adapter.discover_devices().await?;
    pin_mut!(discover);
    while let Some(evt) = discover.next().await {
        match evt {
            AdapterEvent::DeviceAdded(addr) => {
                debug!("Found device: {addr}");
                if addr == to_find {
                    return adapter.device(addr);
                }
            }
            _ => {}
        }
    }

    return Err(bluer::Error {
        kind: bluer::ErrorKind::NotAvailable,
        message: "Requested device not available".to_owned(),
    });
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    env_logger::init();
    let arg_matches = Command::new("hrmon")
        .author("Rintse")
        .about("Log bluetooth heart rate monitor")
        .arg(
            Arg::new("device")
                .help("The bluetooth device's address")
                .required(true)
                .action(clap::ArgAction::Set),
        )
        .get_matches();
    let addr: &String = arg_matches.get_one("device").unwrap();
    let addr_to_find = Address::from_str(addr).unwrap();

    let device = find_device(addr_to_find).await?;
    device.connect().await?;
    hr_loop(device).await
}
