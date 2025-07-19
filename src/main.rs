// 00002a37-0000-1000-8000-00805f9b34fb
// CD:30:22:D6:4F:70

const SERVICE_ID: u16 = 0x000b;
const ATTR_ID: u16 = 0x000c;

use bitfield::bitfield;
use bluer::Address;
use clap::{Arg, Command};
use futures::{StreamExt, pin_mut};
use itertools::Itertools;
use std::str::FromStr;

bitfield! {
    pub struct HRDataFlags(u8);
    impl Debug;

    pub hrv_format_u16, _: 0;
    pub sensor_contact_present, _: 1;
    pub sensor_contact, _: 2;
    pub energy_expended_present, _: 3;
    pub rr_interval_present, _: 4;
}

trait HRDataSize {}

impl HRDataSize for u8 {}
impl HRDataSize for u16 {}

#[derive(Debug)]
struct HRData<T: HRDataSize> {
    contact: Option<bool>,
    hr_measurement: T,
    energy_expended: Option<u16>,
    rr_intervals: Vec<T>,
}

enum HRData2 {
    Single(HRData<u8>),
    Double(HRData<u16>),
}

fn two_u8_to_u16(upper: u8, lower: u8) -> u16 {
    ((upper as u16) << 8) & (lower as u16)
}

type ParseError = &'static str;

impl HRData<u8> {
    fn new(flags: HRDataFlags, v: &[u8]) -> Result<Self, ParseError> {
        assert!(!flags.hrv_format_u16());
        let mut iter = v.iter();

        let hr_measurement = *iter.next().unwrap();

        let contact = if flags.sensor_contact_present() {
            Some(flags.sensor_contact())
        } else {
            None
        };

        let energy_expended = if flags.energy_expended_present() {
            let upper = iter.next().unwrap();
            let lower = iter.next().unwrap();
            Some(two_u8_to_u16(*upper, *lower))
        } else {
            None
        };

        let rr_intervals = if flags.rr_interval_present() {
            iter.cloned().collect()
        } else {
            Vec::new()
        };

        Ok(Self {
            contact,
            hr_measurement,
            energy_expended,
            rr_intervals,
        })
    }
}

impl HRData<u16> {
    fn new(flags: HRDataFlags, v: &[u8]) -> Result<Self, ParseError> {
        assert!(flags.hrv_format_u16());
        let mut iter = v.into_iter();

        let hr_measurement = {
            let upper = *iter.next().unwrap();
            let lower = *iter.next().unwrap();
            two_u8_to_u16(upper, lower)
        };

        let contact = if flags.sensor_contact_present() {
            Some(flags.sensor_contact())
        } else {
            None
        };

        let energy_expended = if flags.energy_expended_present() {
            let upper = iter.next().unwrap();
            let lower = iter.next().unwrap();
            Some(two_u8_to_u16(*upper, *lower))
        } else {
            None
        };

        let rr_intervals = if flags.rr_interval_present() {
            iter.tuple_windows()
                .map(|(u, l)| two_u8_to_u16(*u, *l))
                .collect()
        } else {
            Vec::new()
        };

        Ok(Self {
            contact,
            hr_measurement,
            energy_expended,
            rr_intervals,
        })
    }
}

// https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/HRS_v1.0/out/en/index-en.html
impl TryFrom<Vec<u8>> for HRData2 {
    type Error = ParseError;

    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        let flags_byte = v.first().ok_or("No flags field present")?;
        let flags = HRDataFlags(*flags_byte);

        let hr_data = if flags.hrv_format_u16() {
            Self::Double(HRData::<u16>::new(flags, &v[1..])?)
        } else {
            Self::Single(HRData::<u8>::new(flags, &v[1..])?)
        };

        Ok(hr_data)
    }
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
    let addr = Address::from_str(addr).unwrap();

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let device = adapter.device(addr).unwrap();
    device.connect().await?;

    let service = device.service(SERVICE_ID).await?;
    let characteristic = service.characteristic(ATTR_ID).await?;
    let notify = characteristic.notify().await?;
    pin_mut!(notify);

    for _ in 0..300 {
        match notify.next().await {
            Some(value) => {
                println!("    Notification value: {:x?}", &value);
                match HRData2::try_from(value) {
                    Ok(data) => match data {
                        HRData2::Single(hrdata) => {
                            println!("Single-byte precision HR data: {:?}", hrdata)
                        }
                        HRData2::Double(hrdata) => {
                            println!("Double-byte precision HR data: {:?}", hrdata)
                        }
                    },
                    Err(e) => println!("Could not parse HR data:\n {}", e),
                }
            }
            None => {
                println!("Notification session was terminated");
                break;
            }
        }
    }

    Ok(())
}
