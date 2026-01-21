use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;
use std::path::PathBuf;

use crate::monitor::Monitor;

pub struct BlkInflightMonitor {
    root: PathBuf,
    inflight_read: GaugeVec,  // labels: [device]
    inflight_write: GaugeVec, // labels: [device]
}

impl BlkInflightMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled BlkInflight monitor");

        let inflight_read = GaugeVec::new(
            Opts::new(
                "blkinflight_read",
                "Number of read I/O operations currently in-flight",
            ),
            &["device"],
        )?;
        let inflight_write = GaugeVec::new(
            Opts::new(
                "blkinflight_write",
                "Number of write I/O operations currently in-flight",
            ),
            &["device"],
        )?;

        registry.register(Box::new(inflight_read.clone()))?;
        registry.register(Box::new(inflight_write.clone()))?;

        Ok(Self {
            root: PathBuf::from("/sys/block"),
            inflight_read,
            inflight_write,
        })
    }

    #[inline]
    fn is_partition(path: &std::path::Path) -> bool {
        path.join("partition").exists()
    }

    fn collect_once(&self) -> Result<()> {
        let entries = match fs::read_dir(&self.root) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        let mut dev_count = 0usize;

        for entry in entries.flatten() {
            let dev_name = entry.file_name();
            let dev_str = dev_name.to_string_lossy().to_string();
            let dev_path = entry.path();

            // Skip partitions
            if Self::is_partition(&dev_path) {
                continue;
            }

            // Skip virtual devices
            if dev_str.starts_with("loop") || dev_str.starts_with("ram") {
                continue;
            }

            let inflight_path = dev_path.join("inflight");
            if let Ok(content) = fs::read_to_string(&inflight_path) {
                // Format: "read_count write_count"
                let parts: Vec<&str> = content.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(r) = parts[0].parse::<f64>() {
                        self.inflight_read.with_label_values(&[&dev_str]).set(r);
                    }
                    if let Ok(w) = parts[1].parse::<f64>() {
                        self.inflight_write.with_label_values(&[&dev_str]).set(w);
                    }
                    dev_count += 1;
                }
            }
        }

        debug!("blkinflight: updated {} devices", dev_count);
        Ok(())
    }
}

impl Monitor for BlkInflightMonitor {
    fn name(&self) -> &'static &str {
        &"blkinflight"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
