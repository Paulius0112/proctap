use anyhow::Result;
use log::debug;
use prometheus::{GaugeVec, Opts, Registry};
use std::{fs, path::PathBuf};

use crate::monitor::Monitor;

pub struct NetSysfsStatsMonitor {
    root: PathBuf,
    stats: GaugeVec,
    include_lo: bool,
}

impl NetSysfsStatsMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        let stats = GaugeVec::new(
            Opts::new(
                "netdev_stat",
                "Values from /sys/class/net/<iface>/statistics/* (bytes/packets/errors/drops, etc.)",
            ),
            &["iface", "key"],
        )?;
        registry.register(Box::new(stats.clone()))?;

        Ok(Self {
            root: PathBuf::from("/sys/class/net"),
            stats,
            include_lo: false,
        })
    }

    #[inline]
    fn read_u64(path: &PathBuf) -> Option<u64> {
        let s = fs::read_to_string(path).ok()?;
        s.trim().parse::<u64>().ok()
    }
}

impl Monitor for NetSysfsStatsMonitor {
    fn name(&self) -> &'static &str {
        &"net_sysfs"
    }

    fn collect(&mut self) -> Result<()> {
        let mut if_count = 0usize;
        for entry in fs::read_dir(&self.root)? {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let iface = entry.file_name().to_string_lossy().to_string();

            if !self.include_lo && iface == "lo" {
                continue;
            }

            if !entry.path().join("device").exists() {
                continue;
            }

            let stats_dir = entry.path().join("statistics");
            let Ok(dir_iter) = fs::read_dir(&stats_dir) else {
                continue;
            };

            for stat in dir_iter {
                let stat = match stat {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let key = stat.file_name().to_string_lossy().to_string();
                let path = stat.path();

                if let Some(val) = Self::read_u64(&path) {
                    self.stats
                        .with_label_values(&[iface.as_str(), key.as_str()])
                        .set(val as f64);
                }
            }

            if_count += 1;
        }

        debug!("net_sysfs_stats: updated stats for {if_count} interfaces");
        Ok(())
    }
}
