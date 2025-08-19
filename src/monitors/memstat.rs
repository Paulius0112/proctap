use anyhow::Result;
use log::debug;
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;

use crate::monitor::Monitor;

/// Exposes /proc/meminfo as:
///   meminfo_bytes{key="<...>"}  <bytes>   (for lines ending with kB)
///   meminfo{key="<...>"}        <value>   (unitless counters)
pub struct MeminfoMonitor {
    bytes: GaugeVec, // labels: key
    other: GaugeVec, // labels: key
}

impl MeminfoMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        let bytes = GaugeVec::new(
            Opts::new("meminfo_bytes", "Meminfo entries reported in kB, converted to bytes"),
            &["key"],
        )?;
        let other = GaugeVec::new(
            Opts::new("meminfo", "Meminfo entries without kB units (counts)"),
            &["key"],
        )?;
        registry.register(Box::new(bytes.clone()))?;
        registry.register(Box::new(other.clone()))?;
        Ok(Self { bytes, other })
    }

    fn collect_once(&self) -> Result<()> {
        let s = fs::read_to_string("/proc/meminfo")?;
        let mut seen = 0usize;

        for line in s.lines() {
            // e.g. "MemTotal:       16336236 kB"
            let (k, rest) = match line.split_once(':') {
                Some((k, v)) => (k.trim(), v.trim()),
                None => continue,
            };

            // Split remaining by whitespace to get value and optional unit
            let mut it = rest.split_whitespace();
            let v = match it.next() {
                Some(x) => x,
                None => continue,
            };

            if let Ok(num) = v.parse::<u64>() {
                match it.next() {
                    Some("kB") => {
                        // convert to bytes
                        self.bytes.with_label_values(&[k]).set((num * 1024) as f64);
                    }
                    _ => {
                        // unitless counters (HugePages_*, etc.)
                        self.other.with_label_values(&[k]).set(num as f64);
                    }
                }
                seen += 1;
            }
        }

        debug!("meminfo: updated {seen} keys");
        Ok(())
    }
}

impl Monitor for MeminfoMonitor {
    fn name(&self) -> &'static &str {
        &"meminfo"
    }
    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
