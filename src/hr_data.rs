use std::u16;

use bitfield::bitfield;
use itertools::Itertools;
use log::debug;

bitfield! {
    struct HRDataFlags(u8);
    impl Debug;
    pub hrv_format_is_u16, _: 0;
    pub sensor_contact, _: 1;
    pub sensor_contact_present, _: 2;
    pub energy_expended_present, _: 3;
    pub rr_interval_present, _: 4;
}

pub trait HRDataSize {}

impl HRDataSize for u8 {}
impl HRDataSize for u16 {}

#[derive(Debug)]
pub struct HRData<T: HRDataSize> {
    pub contact: Option<bool>,
    pub hr_measurement: T,
    pub energy_expended: Option<u16>,
    pub rr_intervals: Vec<f64>,
}

// TODO: how to name this thing
pub enum HRData2 {
    Single(HRData<u8>),
    Double(HRData<u16>),
}

type ParseError = &'static str;

// TODO: lots of duplication between this and the u16 variant
impl HRData<u8> {
    fn new(flags: HRDataFlags, v: &[u8]) -> Result<Self, ParseError> {
        assert!(!flags.hrv_format_is_u16());
        let mut iter = v.iter();

        let hr_measurement = *iter.next().unwrap();

        let contact = if flags.sensor_contact_present() {
            Some(flags.sensor_contact())
        } else {
            None
        };

        let energy_expended = if flags.energy_expended_present() {
            Some(u16::from_le_bytes([
                *iter.next().unwrap(),
                *iter.next().unwrap(),
            ]))
        } else {
            None
        };

        let rr_intervals = if flags.rr_interval_present() {
            iter.cloned().tuple_windows().map(
                |(u, l)| u16::from_le_bytes([u, l]) as f64 / 1024.0
            ).collect()
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
        assert!(flags.hrv_format_is_u16());
        let mut iter = v.into_iter();

        // TODO: order
        let hr_measurement =
            u16::from_le_bytes([*iter.next().unwrap(), *iter.next().unwrap()]);

        let contact = if flags.sensor_contact_present() {
            Some(flags.sensor_contact())
        } else {
            None
        };

        let energy_expended = if flags.energy_expended_present() {
            Some(u16::from_le_bytes([
                *iter.next().unwrap(),
                *iter.next().unwrap(),
            ]))
        } else {
            None
        };

        let rr_intervals = if flags.rr_interval_present() {
            iter.cloned().tuple_windows().map(
                |(u, l)| u16::from_le_bytes([u, l]) as f64 / 1024.0
            ).collect()
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

        debug!("HR data flags: {flags:?}");

        let hr_data = if flags.hrv_format_is_u16() {
            Self::Double(HRData::<u16>::new(flags, &v[1..])?)
        } else {
            Self::Single(HRData::<u8>::new(flags, &v[1..])?)
        };

        Ok(hr_data)
    }
}
