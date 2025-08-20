use anyhow::Result;
use log::debug;
use prometheus::{GaugeVec, Opts, Registry};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::monitor::Monitor;

// Exposes /sys/class/block/<dev>/stat as:
//   disk_stat{dev="<dev>", key="<field>"} <value>
// Base fields (kernel docs: Documentation/admin-guide/iostats.rst):

pub struct DiskStatsMonitor {
    root: PathBuf,
    stats: GaugeVec,
    // Skip partition
    pub include_partitions: bool,
    pub skip_virtual: bool,
}

impl DiskStatsMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        let stats = GaugeVec::new(
            Opts::new("disk_stat", "Values from /sys/class/block/<dev>/stat (iostats)"),
            &["dev", "key"],
        )?;
        registry.register(Box::new(stats.clone()))?;

        Ok(Self {
            root: PathBuf::from("/sys/class/block"),
            stats,
            include_partitions: false,
            skip_virtual: true,
        })
    }

    #[inline]
    fn read_stat_file(path: &PathBuf) -> Option<Vec<u64>> {
        let s = fs::read_to_string(path).ok()?;
        let mut out = Vec::with_capacity(17);
        for tok in s.split_whitespace() {
            if let Ok(v) = tok.parse::<u64>() {
                out.push(v);
            } else {
                return None;
            }
        }
        Some(out)
    }

    #[inline]
    fn is_partition(dev_path: &Path) -> bool {
        dev_path.join("partition").exists()
    }

    #[inline]
    fn is_virtual_like(dev_name: &str) -> bool {
        dev_name.starts_with("loop") || dev_name.starts_with("ram") || dev_name.starts_with("dm-")
    }
}

impl Monitor for DiskStatsMonitor {
    fn name(&self) -> &'static &str {
        &"diskstats"
    }

    fn collect(&mut self) -> Result<()> {
        let mut count = 0usize;

        for entry in fs::read_dir(&self.root)? {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let dev = entry.file_name().to_string_lossy().to_string();
            let dev_path = entry.path();

            if self.skip_virtual && Self::is_virtual_like(&dev) {
                continue;
            }
            if !self.include_partitions && Self::is_partition(&dev_path) {
                continue;
            }

            let stat_path = dev_path.join("stat");
            let Some(vals) = Self::read_stat_file(&stat_path) else {
                continue;
            };

            let mut keys = vec![
                "reads_completed",
                "reads_merged",
                "sectors_read",
                "read_time_ms",
                "writes_completed",
                "writes_merged",
                "sectors_written",
                "write_time_ms",
                "io_in_progress",
                "io_time_ms",
                "weighted_io_time_ms",
            ];

            // Need to be able to handle multiple kernel versions
            // which provide additional metrics
            if vals.len() >= 15 {
                keys.extend_from_slice(&[
                    "discards_completed",
                    "discards_merged",
                    "sectors_discarded",
                    "discard_time_ms",
                ]);
            }
            if vals.len() >= 17 {
                keys.extend_from_slice(&["flush_requests_completed", "flush_time_ms"]);
            }

            for (i, key) in keys.iter().enumerate() {
                if let Some(v) = vals.get(i) {
                    self.stats.with_label_values(&[dev.as_str(), key]).set(*v as f64);
                }
            }

            count += 1;
        }

        debug!("diskstats: updated {count} devices");
        Ok(())
    }
}
