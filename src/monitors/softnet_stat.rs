use anyhow::{Context, Result};
use log::debug;
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;

use crate::monitor::Monitor;

pub struct SoftnetStatMonitor {
    metric: GaugeVec,
}

impl SoftnetStatMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        let metric = GaugeVec::new(
            Opts::new(
                "softnet_stat",
                "Per-CPU hex counters from /proc/net/softnet_stat (RX path health)",
            ),
            &["cpu", "key"],
        )?;
        registry.register(Box::new(metric.clone()))?;
        Ok(Self { metric })
    }

    #[inline]
    fn set_named_and_indexed(&self, cpu_s: &str, idx: usize, val: u64) {
        match idx {
            0 => {
                self.metric.with_label_values(&[cpu_s, "processed"]).set(val as f64);
            }
            1 => {
                self.metric.with_label_values(&[cpu_s, "dropped"]).set(val as f64);
            }
            2 => {
                self.metric.with_label_values(&[cpu_s, "time_squeezed"]).set(val as f64);
            }
            _ => {}
        }

        let key = format!("f{idx}");
        self.metric.with_label_values(&[cpu_s, &key]).set(val as f64);
    }
}

impl Monitor for SoftnetStatMonitor {
    fn name(&self) -> &'static &str {
        &"softnet_stat"
    }

    fn collect(&mut self) -> Result<()> {
        let s = fs::read_to_string("/proc/net/softnet_stat").context("reading /proc/net/softnet_stat")?;

        let mut cpu_count = 0usize;
        for (cpu_idx, line) in s.lines().enumerate() {
            let cpu_s = cpu_idx.to_string();

            for (i, tok) in line.split_whitespace().enumerate() {
                if let Ok(v) = u64::from_str_radix(tok, 16) {
                    self.set_named_and_indexed(&cpu_s, i, v);
                }
            }
            cpu_count += 1;
        }

        debug!("softnet_stat: updated {cpu_count} CPUs");
        Ok(())
    }
}
