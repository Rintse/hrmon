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

#[derive(Debug)]
pub struct HRData {
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
        let flags_byte = v.first().ok_or("No flags field present")?;
        let flags = HRDataFlags(*flags_byte);
        debug!("HR data flags: {flags:?}");
        let mut iter = v.into_iter();

        let hr_measurement = if flags.hrv_format_is_u16() {
            u16::from_le_bytes([iter.next().unwrap(), iter.next().unwrap()])
        } else {
            iter.next().unwrap() as u16
        };

        let contact = if flags.sensor_contact_present() {
            Some(flags.sensor_contact())
        } else {
            None
        };

        let energy_expended = if flags.energy_expended_present() {
            Some(u16::from_le_bytes([
                iter.next().unwrap(),
                iter.next().unwrap(),
            ]))
        } else {
            None
        };

        let rr_intervals = if flags.rr_interval_present() {
            iter.tuple_windows().map(
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
