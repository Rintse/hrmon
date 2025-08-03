#[path = "../hr_data.rs"]
mod hr_data;

use clap::Parser;
use hr_data::HRData;
use log::{info, warn};
use metrics::{describe_gauge, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::{io::BufRead, net::SocketAddr};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CliArgs {
    bind_addr: SocketAddr,
}

fn main() {
    env_logger::init();
    let args = CliArgs::parse();
    let builder = PrometheusBuilder::new().with_http_listener(args.bind_addr);
    builder.install().expect("failed to install recorder/exporter");
    info!("Exporter installed");

    describe_gauge!("hr_rate", "Latest heart rate measurmenent (BPM)");
    let gauge_hr_rate = gauge!("hr_rate");
    describe_gauge!("rr_interval", "Latest RR-interval (ms)");
    let gauge_rr_interval = gauge!("rr_interval");

    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        if let Ok(data) = serde_json::from_str::<HRData>(&line) {
            if let Some(c) = data.contact && !c {
                info!("Skipping HR measurment without sensor contact");
                continue;
            }

            info!("Setting data: {data:?}");
            gauge_hr_rate.set(f64::from(data.hr_measurement));
            if let Some(rr) = data.rr_intervals.last() {
                gauge_rr_interval.set(*rr);
            }
        } else {
            warn!("Could not parse HR data");
        }
    }
}
