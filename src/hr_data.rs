use std::time::{SystemTime, UNIX_EPOCH};

use bitfield::bitfield;
use log::debug;
use serde::Serialize;

bitfield! {
    struct HRDataFlags(u8);
    impl Debug;
    pub hrv_format_is_u16, _: 0;
    pub sensor_contact, _: 1;
    pub sensor_contact_present, _: 2;
    pub energy_expended_present, _: 3;
    pub rr_interval_present, _: 4;
}

#[derive(Debug, Serialize)]
pub struct HRData {
    pub timestamp: u128, // NOTE: this is processing time, not measurement time
    pub hr_measurement: u16,
    pub contact: Option<bool>,
    pub energy_expended: Option<u16>,
    pub rr_intervals: Vec<f64>,
}

type ParseError = &'static str;

// https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/HRS_v1.0/out/en/index-en.html
impl TryFrom<Vec<u8>> for HRData {
    type Error = ParseError;

    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("we should be after the epoch")
            .as_millis();
        let flags = HRDataFlags(v[0]);
        debug!("HR data flags: {flags:?}");
        let mut i = 1;

        let hr_measurement = if flags.hrv_format_is_u16() {
            i += 2;
            u16::from_le_bytes([v[i - 2], v[i - 1]])
        } else {
            i += 1;
            v[i - 1] as u16
        };

        let contact = if flags.sensor_contact_present() {
            Some(flags.sensor_contact())
        } else {
            None
        };

        let energy_expended = if flags.energy_expended_present() {
            i += 2;
            Some(u16::from_le_bytes([v[i - 2], v[i - 1]]))
        } else {
            None
        };

        let rr_intervals = if flags.rr_interval_present() {
            let mut rrs = Vec::new();
            while i < v.len() {
                i += 2;
                rrs.push(
                    u16::from_le_bytes([v[i - 2], v[i - 1]]) as f64 / 1024.0,
                )
            }
            rrs
        } else {
            Vec::new()
        };

        Ok(Self {
            timestamp,
            contact,
            hr_measurement,
            energy_expended,
            rr_intervals,
        })
    }
}
