use anyhow::Result;
use log::{debug, info};
use prometheus::{Gauge, Registry};
use std::fs;

use crate::monitor::Monitor;

pub struct UptimeMonitor {
    uptime_seconds: Gauge,
    idle_seconds: Gauge,
}

impl UptimeMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled Uptime monitor");

        let uptime_seconds = Gauge::new("uptime_seconds", "System uptime in seconds")?;
        let idle_seconds = Gauge::new(
            "uptime_idle_seconds",
            "Total time all CPUs have spent idle in seconds",
        )?;

        registry.register(Box::new(uptime_seconds.clone()))?;
        registry.register(Box::new(idle_seconds.clone()))?;

        Ok(Self {
            uptime_seconds,
            idle_seconds,
        })
    }

    fn collect_once(&self) -> Result<()> {
        // Format: "12345.67 98765.43"
        let content = fs::read_to_string("/proc/uptime")?;
        let parts: Vec<&str> = content.split_whitespace().collect();

        if parts.len() >= 2 {
            if let Ok(v) = parts[0].parse::<f64>() {
                self.uptime_seconds.set(v);
            }
            if let Ok(v) = parts[1].parse::<f64>() {
                self.idle_seconds.set(v);
            }
        }

        debug!("uptime: updated");
        Ok(())
    }
}

impl Monitor for UptimeMonitor {
    fn name(&self) -> &'static &str {
        &"uptime"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
