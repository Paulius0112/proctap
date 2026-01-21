use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;
use std::path::PathBuf;

use crate::monitor::Monitor;

pub struct BlkQueueMonitor {
    root: PathBuf,
    nr_requests: GaugeVec,     // labels: [device]
    read_ahead_kb: GaugeVec,   // labels: [device]
    rotational: GaugeVec,      // labels: [device]
    hw_sector_size: GaugeVec,  // labels: [device]
    logical_block_size: GaugeVec,  // labels: [device]
    physical_block_size: GaugeVec, // labels: [device]
    max_sectors_kb: GaugeVec,  // labels: [device]
    max_hw_sectors_kb: GaugeVec, // labels: [device]
}

impl BlkQueueMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled BlkQueue monitor");

        let nr_requests = GaugeVec::new(
            Opts::new(
                "blkqueue_nr_requests",
                "Number of requests in the block device queue",
            ),
            &["device"],
        )?;
        let read_ahead_kb = GaugeVec::new(
            Opts::new(
                "blkqueue_read_ahead_kb",
                "Read-ahead size in KB",
            ),
            &["device"],
        )?;
        let rotational = GaugeVec::new(
            Opts::new(
                "blkqueue_rotational",
                "Whether the device is rotational (1) or not (0)",
            ),
            &["device"],
        )?;
        let hw_sector_size = GaugeVec::new(
            Opts::new(
                "blkqueue_hw_sector_size_bytes",
                "Hardware sector size in bytes",
            ),
            &["device"],
        )?;
        let logical_block_size = GaugeVec::new(
            Opts::new(
                "blkqueue_logical_block_size_bytes",
                "Logical block size in bytes",
            ),
            &["device"],
        )?;
        let physical_block_size = GaugeVec::new(
            Opts::new(
                "blkqueue_physical_block_size_bytes",
                "Physical block size in bytes",
            ),
            &["device"],
        )?;
        let max_sectors_kb = GaugeVec::new(
            Opts::new(
                "blkqueue_max_sectors_kb",
                "Maximum request size in KB",
            ),
            &["device"],
        )?;
        let max_hw_sectors_kb = GaugeVec::new(
            Opts::new(
                "blkqueue_max_hw_sectors_kb",
                "Maximum hardware request size in KB",
            ),
            &["device"],
        )?;

        registry.register(Box::new(nr_requests.clone()))?;
        registry.register(Box::new(read_ahead_kb.clone()))?;
        registry.register(Box::new(rotational.clone()))?;
        registry.register(Box::new(hw_sector_size.clone()))?;
        registry.register(Box::new(logical_block_size.clone()))?;
        registry.register(Box::new(physical_block_size.clone()))?;
        registry.register(Box::new(max_sectors_kb.clone()))?;
        registry.register(Box::new(max_hw_sectors_kb.clone()))?;

        Ok(Self {
            root: PathBuf::from("/sys/block"),
            nr_requests,
            read_ahead_kb,
            rotational,
            hw_sector_size,
            logical_block_size,
            physical_block_size,
            max_sectors_kb,
            max_hw_sectors_kb,
        })
    }

    fn read_value(path: &std::path::Path) -> Option<f64> {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
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

            // Skip virtual devices like loop, ram
            if dev_str.starts_with("loop") || dev_str.starts_with("ram") {
                continue;
            }

            let queue_path = dev_path.join("queue");
            if !queue_path.exists() {
                continue;
            }

            if let Some(v) = Self::read_value(&queue_path.join("nr_requests")) {
                self.nr_requests.with_label_values(&[&dev_str]).set(v);
            }
            if let Some(v) = Self::read_value(&queue_path.join("read_ahead_kb")) {
                self.read_ahead_kb.with_label_values(&[&dev_str]).set(v);
            }
            if let Some(v) = Self::read_value(&queue_path.join("rotational")) {
                self.rotational.with_label_values(&[&dev_str]).set(v);
            }
            if let Some(v) = Self::read_value(&queue_path.join("hw_sector_size")) {
                self.hw_sector_size.with_label_values(&[&dev_str]).set(v);
            }
            if let Some(v) = Self::read_value(&queue_path.join("logical_block_size")) {
                self.logical_block_size.with_label_values(&[&dev_str]).set(v);
            }
            if let Some(v) = Self::read_value(&queue_path.join("physical_block_size")) {
                self.physical_block_size.with_label_values(&[&dev_str]).set(v);
            }
            if let Some(v) = Self::read_value(&queue_path.join("max_sectors_kb")) {
                self.max_sectors_kb.with_label_values(&[&dev_str]).set(v);
            }
            if let Some(v) = Self::read_value(&queue_path.join("max_hw_sectors_kb")) {
                self.max_hw_sectors_kb.with_label_values(&[&dev_str]).set(v);
            }

            dev_count += 1;
        }

        debug!("blkqueue: updated {} devices", dev_count);
        Ok(())
    }
}

impl Monitor for BlkQueueMonitor {
    fn name(&self) -> &'static &str {
        &"blkqueue"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
