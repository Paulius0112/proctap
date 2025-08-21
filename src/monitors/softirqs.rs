use anyhow::{Context, Result};
use log::debug;
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;

use crate::monitor::Monitor;

pub struct SoftirqsMonitor {
    metric: GaugeVec,
}

impl SoftirqsMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        let metric = GaugeVec::new(
            Opts::new("softirqs", "Per-CPU softirq counters from /proc/softirqs"),
            &["kind", "cpu"],
        )?;
        registry.register(Box::new(metric.clone()))?;
        Ok(Self { metric })
    }
}

impl Monitor for SoftirqsMonitor {
    fn name(&self) -> &'static &str {
        &"softirqs"
    }

    fn collect(&mut self) -> Result<()> {
        let s = fs::read_to_string("/proc/softirqs").context("reading /proc/softirqs")?;
        let mut lines = s.lines();

        let header = lines.next().unwrap_or("");
        let ncpus_header = header.split_whitespace().filter(|t| t.starts_with("CPU")).count();

        let mut rows = 0usize;

        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let (kind, rest) = match line.split_once(':') {
                Some((k, r)) => (k.trim(), r.trim()),
                None => continue,
            };

            for (cpu_idx, val_s) in rest.split_whitespace().enumerate() {
                if let Ok(v) = val_s.parse::<u64>() {
                    self.metric
                        .with_label_values(&[kind, &cpu_idx.to_string()])
                        .set(v as f64);
                }
            }

            rows += 1;
        }

        debug!("softirqs: updated {rows} kinds across ~{ncpus_header} CPUs");
        Ok(())
    }
}
