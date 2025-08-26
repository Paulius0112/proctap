use anyhow::{Context, Result};
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::{fs, path::PathBuf, process::Command};

use crate::monitor::Monitor;

pub struct EthtoolMonitor {
    stats: GaugeVec,
}

impl EthtoolMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled Ethtool monitor");
        let stats = GaugeVec::new(
            Opts::new("ethtool_stat", "Values from `ethtool -S <iface>`"),
            &["iface", "key"],
        )?;
        registry.register(Box::new(stats.clone()))?;
        Ok(Self { stats })
    }

    fn scrape_iface(&self, iface: &str) -> Result<usize> {
        let out = Command::new("ethtool")
            .args(["-S", iface])
            .output()
            .with_context(|| format!("running `ethtool -S {iface}`"))?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            // If interface doesn't support -S
            debug!("ethtool -S {iface} failed: {}", stderr.trim());
            return Ok(0);
        }

        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut lines = stdout.lines();

        if let Some(first) = lines.clone().next() {
            if first.to_ascii_lowercase().contains("statistics") {
                lines.next();
            }
        }

        let mut n = 0usize;
        for line in lines {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }

            if let Some((k, v)) = l.split_once(':') {
                let key = k.trim();
                let val_s = v.trim();
                // Record only numeric
                if let Ok(val) = val_s.parse::<f64>() {
                    self.stats.with_label_values(&[iface, key]).set(val);
                    n += 1;
                }
            }
        }
        Ok(n)
    }
}

impl Monitor for EthtoolMonitor {
    fn name(&self) -> &'static &str {
        &"ethtool"
    }

    fn collect(&mut self) -> Result<()> {
        let root = PathBuf::from("/sys/class/net");
        let entries = fs::read_dir(&root).with_context(|| format!("reading {root:?}"))?;

        let mut ifaces = 0usize;
        let mut rows = 0usize;

        for e in entries {
            let e = e?;
            let iface = e.file_name().to_string_lossy().to_string();
            if iface == "lo" {
                continue;
            }
            rows += self.scrape_iface(&iface)?;
            ifaces += 1;
        }

        debug!("ethtool: updated {rows} rows across {ifaces} interfaces");
        Ok(())
    }
}
