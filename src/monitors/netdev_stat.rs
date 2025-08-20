use anyhow::{Context, Result};
use log::{debug, error};
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
    fn read_u64(path: &PathBuf) -> anyhow::Result<u64> {
        let s = fs::read_to_string(path).with_context(|| format!("reading {path:?}"))?;
        let v = s
            .trim()
            .parse::<u64>()
            .with_context(|| format!("parsing u64 from {path:?}"))?;
        Ok(v)
    }
}

impl Monitor for NetSysfsStatsMonitor {
    fn name(&self) -> &'static &str {
        &"net_sysfs"
    }

    fn collect(&mut self) -> Result<()> {
        let mut if_count = 0usize;

        let entries =
            fs::read_dir(&self.root).with_context(|| format!("reading net class directory: {:?}", self.root))?;

        for entry_res in entries {
            let entry = entry_res.with_context(|| "iterating /sys/class/net entries".to_string())?;

            let iface = entry.file_name().to_string_lossy().to_string();

            if !self.include_lo && iface == "lo" {
                continue;
            }

            if !entry.path().join("device").exists() {
                continue;
            }

            let stats_dir = entry.path().join("statistics");
            let dir_iter = fs::read_dir(&stats_dir)
                .with_context(|| format!("reading statistics directory for {iface}: {stats_dir:?}"))?;

            for stat_res in dir_iter {
                let stat = stat_res.with_context(|| format!("reading stat entry in {iface} ({stats_dir:?})"))?;
                let key = stat.file_name().to_string_lossy().to_string();
                let path = stat.path();

                match Self::read_u64(&path) {
                    Ok(val) => {
                        self.stats
                            .with_label_values(&[iface.as_str(), key.as_str()])
                            .set(val as f64);
                    }
                    Err(e) => {
                        error!("net_sysfs_stats: failed to read {iface}/{key} at {path:?}: {e:#}");
                        return Err(e);
                    }
                }
            }

            if_count += 1;
        }

        debug!("net_sysfs_stats: updated stats for {if_count} interfaces");
        Ok(())
    }
}
