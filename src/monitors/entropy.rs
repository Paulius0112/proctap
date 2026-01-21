use anyhow::Result;
use log::{debug, info};
use prometheus::{Gauge, Registry};
use std::fs;
use std::path::Path;

use crate::monitor::Monitor;

pub struct EntropyMonitor {
    entropy_available: Gauge,
    poolsize: Gauge,
}

impl EntropyMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled Entropy monitor");

        let entropy_available = Gauge::new(
            "entropy_available_bits",
            "Number of bits of entropy available in the random pool",
        )?;
        let poolsize = Gauge::new(
            "entropy_poolsize_bits",
            "Size of the entropy pool in bits",
        )?;

        registry.register(Box::new(entropy_available.clone()))?;
        registry.register(Box::new(poolsize.clone()))?;

        Ok(Self {
            entropy_available,
            poolsize,
        })
    }

    fn read_value(path: &Path) -> Option<f64> {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
    }

    fn collect_once(&self) -> Result<()> {
        let base = Path::new("/proc/sys/kernel/random");

        if let Some(v) = Self::read_value(&base.join("entropy_avail")) {
            self.entropy_available.set(v);
        }
        if let Some(v) = Self::read_value(&base.join("poolsize")) {
            self.poolsize.set(v);
        }

        debug!("entropy: updated");
        Ok(())
    }
}

impl Monitor for EntropyMonitor {
    fn name(&self) -> &'static &str {
        &"entropy"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
