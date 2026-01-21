use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;
use std::path::PathBuf;

use crate::monitor::Monitor;

pub struct NetInfoMonitor {
    root: PathBuf,
    speed: GaugeVec,     // labels: [iface]
    mtu: GaugeVec,       // labels: [iface]
    carrier: GaugeVec,   // labels: [iface]
    flags: GaugeVec,     // labels: [iface]
    tx_queue_len: GaugeVec, // labels: [iface]
    operstate: GaugeVec, // labels: [iface, state]
}

impl NetInfoMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled NetInfo monitor");

        let speed = GaugeVec::new(
            Opts::new(
                "netinfo_speed_mbps",
                "Network interface speed in Mbps",
            ),
            &["iface"],
        )?;
        let mtu = GaugeVec::new(
            Opts::new(
                "netinfo_mtu_bytes",
                "Network interface MTU in bytes",
            ),
            &["iface"],
        )?;
        let carrier = GaugeVec::new(
            Opts::new(
                "netinfo_carrier",
                "Network interface carrier state (1=up, 0=down)",
            ),
            &["iface"],
        )?;
        let flags = GaugeVec::new(
            Opts::new(
                "netinfo_flags",
                "Network interface flags (hex value)",
            ),
            &["iface"],
        )?;
        let tx_queue_len = GaugeVec::new(
            Opts::new(
                "netinfo_tx_queue_len",
                "Network interface transmit queue length",
            ),
            &["iface"],
        )?;
        let operstate = GaugeVec::new(
            Opts::new(
                "netinfo_operstate",
                "Network interface operational state (1 if in this state)",
            ),
            &["iface", "state"],
        )?;

        registry.register(Box::new(speed.clone()))?;
        registry.register(Box::new(mtu.clone()))?;
        registry.register(Box::new(carrier.clone()))?;
        registry.register(Box::new(flags.clone()))?;
        registry.register(Box::new(tx_queue_len.clone()))?;
        registry.register(Box::new(operstate.clone()))?;

        Ok(Self {
            root: PathBuf::from("/sys/class/net"),
            speed,
            mtu,
            carrier,
            flags,
            tx_queue_len,
            operstate,
        })
    }

    fn read_value(path: &std::path::Path) -> Option<f64> {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
    }

    fn read_hex_value(path: &std::path::Path) -> Option<f64> {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| {
                let s = s.trim().trim_start_matches("0x");
                u64::from_str_radix(s, 16).ok()
            })
            .map(|v| v as f64)
    }

    fn read_string(path: &std::path::Path) -> Option<String> {
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    fn collect_once(&self) -> Result<()> {
        let entries = match fs::read_dir(&self.root) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        let mut iface_count = 0usize;

        for entry in entries.flatten() {
            let iface_name = entry.file_name();
            let iface_str = iface_name.to_string_lossy().to_string();
            let iface_path = entry.path();

            // Read speed (may not exist for all interfaces)
            if let Some(v) = Self::read_value(&iface_path.join("speed")) {
                // speed is in Mbps, -1 means unknown
                if v >= 0.0 {
                    self.speed.with_label_values(&[&iface_str]).set(v);
                }
            }

            if let Some(v) = Self::read_value(&iface_path.join("mtu")) {
                self.mtu.with_label_values(&[&iface_str]).set(v);
            }

            if let Some(v) = Self::read_value(&iface_path.join("carrier")) {
                self.carrier.with_label_values(&[&iface_str]).set(v);
            }

            if let Some(v) = Self::read_hex_value(&iface_path.join("flags")) {
                self.flags.with_label_values(&[&iface_str]).set(v);
            }

            if let Some(v) = Self::read_value(&iface_path.join("tx_queue_len")) {
                self.tx_queue_len.with_label_values(&[&iface_str]).set(v);
            }

            // Operstate as labeled metric
            if let Some(state) = Self::read_string(&iface_path.join("operstate")) {
                self.operstate.with_label_values(&[&iface_str, &state]).set(1.0);
            }

            iface_count += 1;
        }

        debug!("netinfo: updated {} interfaces", iface_count);
        Ok(())
    }
}

impl Monitor for NetInfoMonitor {
    fn name(&self) -> &'static &str {
        &"netinfo"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
