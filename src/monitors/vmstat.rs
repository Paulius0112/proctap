use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;

use crate::monitor::Monitor;

pub struct VmStatMonitor {
    vmstat: GaugeVec, // labels: [key]
}

impl VmStatMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled VmStat monitor");

        let vmstat = GaugeVec::new(
            Opts::new("vmstat", "Virtual memory statistics from /proc/vmstat"),
            &["key"],
        )?;
        registry.register(Box::new(vmstat.clone()))?;

        Ok(Self { vmstat })
    }

    fn collect_once(&self) -> Result<()> {
        let content = fs::read_to_string("/proc/vmstat")?;
        let mut seen = 0usize;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let key = parts[0];
                if let Ok(val) = parts[1].parse::<u64>() {
                    self.vmstat.with_label_values(&[key]).set(val as f64);
                    seen += 1;
                }
            }
        }

        debug!("vmstat: updated {} keys", seen);
        Ok(())
    }
}

impl Monitor for VmStatMonitor {
    fn name(&self) -> &'static &str {
        &"vmstat"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
