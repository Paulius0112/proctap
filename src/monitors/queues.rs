use anyhow::{Context, Result};
use log::debug;
use prometheus::{GaugeVec, Opts, Registry};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::monitor::Monitor;

pub struct NetSysfsQueuesMonitor {
    root: PathBuf,
    metrics: GaugeVec,
    include_lo: bool,
}

impl NetSysfsQueuesMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        let metrics = GaugeVec::new(
            Opts::new(
                "netdev_queue_stat",
                "Numeric values from /sys/class/net/<iface>/queues/{rx|tx}-<qid>/*",
            ),
            &["iface", "qtype", "qid", "key"],
        )?;
        registry.register(Box::new(metrics.clone()))?;

        Ok(Self {
            root: PathBuf::from("/sys/class/net"),
            metrics,
            include_lo: false,
        })
    }

    #[inline]
    fn read_u64(path: &Path) -> Result<u64> {
        let s = fs::read_to_string(path).with_context(|| format!("reading {path:?}"))?;
        let s = s.trim();
        let v = s
            .parse::<u64>()
            .with_context(|| format!("parsing decimal u64 from {path:?} (got: {s})"))?;
        Ok(v)
    }

    #[inline]
    fn emit_file(&self, iface: &str, qtype: &str, qid: &str, key: &str, path: &Path) {
        match Self::read_u64(path) {
            Ok(val) => {
                self.metrics
                    .with_label_values(&[iface, qtype, qid, key])
                    .set(val as f64);
            }
            Err(e) => {
                debug!("net_sysfs_queues: skip {path:?}: {e:#}");
            }
        }
    }

    fn scrape_queue_dir(&self, iface: &str, qtype: &str, qid: &str, qdir: &Path) -> Result<usize> {
        let mut count = 0usize;
        let entries =
            fs::read_dir(qdir).with_context(|| format!("reading queue dir {qdir:?} ({qtype}-{qid})"))?;

        for entry_res in entries {
            let entry = entry_res?;
            let path = entry.path();
            let ft = entry
                .file_type()
                .with_context(|| format!("reading file_type for {path:?}"))?;
            let name = entry.file_name().to_string_lossy().to_string();

            if ft.is_file() {
                self.emit_file(iface, qtype, qid, &name, &path);
                count += 1;
            } else if ft.is_dir() {
                let subdir = path;
                let sub_entries = match fs::read_dir(&subdir) {
                    Ok(it) => it,
                    Err(e) => {
                        debug!("net_sysfs_queues: cannot read subdir {subdir:?}: {e}");
                        continue;
                    }
                };
                for sub_res in sub_entries {
                    let sub = match sub_res {
                        Ok(s) => s,
                        Err(e) => {
                            debug!("net_sysfs_queues: iter subdir {subdir:?}: {e}");
                            continue;
                        }
                    };
                    let sub_path = sub.path();
                    let sub_ft = match sub.file_type() {
                        Ok(ft) => ft,
                        Err(e) => {
                            debug!("net_sysfs_queues: file_type {sub_path:?}: {e}");
                            continue;
                        }
                    };
                    if !sub_ft.is_file() {
                        continue;
                    }
                    let sub_name = format!("{}_{}", name, sub.file_name().to_string_lossy());
                    self.emit_file(iface, qtype, qid, &sub_name, &sub_path);
                    count += 1;
                }
            } else {
                continue;
            }
        }

        Ok(count)
    }
}

impl Monitor for NetSysfsQueuesMonitor {
    fn name(&self) -> &'static &str {
        &"net_sysfs_queues"
    }

    fn collect(&mut self) -> Result<()> {
        let mut if_count = 0usize;
        let mut q_count = 0usize;

        let entries =
            fs::read_dir(&self.root).with_context(|| format!("reading net class directory: {:?}", self.root))?;

        for entry_res in entries {
            let entry = entry_res.with_context(|| "iterating /sys/class/net entries".to_string())?;
            let iface = entry.file_name().to_string_lossy().to_string();

            if !self.include_lo && iface == "lo" {
                continue;
            }

            let queues_dir = entry.path().join("queues");
            let Ok(qiter) = fs::read_dir(&queues_dir) else {
                debug!("net_sysfs_queues: no queues for {iface} at {queues_dir:?}");
                continue;
            };

            for q_res in qiter {
                let q = q_res?;
                let qname = q.file_name().to_string_lossy().to_string();

                let (qtype, qid) = match qname.split_once('-') {
                    Some((qt, id)) if (qt == "rx" || qt == "tx") && !id.is_empty() => (qt, id),
                    _ => {
                        debug!("net_sysfs_queues: unknown queue name '{qname}' for iface {iface}");
                        continue;
                    }
                };

                let qdir = q.path();
                let added = self.scrape_queue_dir(&iface, qtype, qid, &qdir)?;
                if added > 0 {
                    q_count += 1;
                }
            }

            if_count += 1;
        }

        debug!(
            "net_sysfs_queues: updated {if_count} ifaces, {q_count} queues (numeric files only)"
        );
        Ok(())
    }
}
