use anyhow::Result;
use log::debug;
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;

use crate::monitor::Monitor;

pub struct InterruptsMonitor {
    metric: GaugeVec, // labels: irq, cpu, name
}

impl InterruptsMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        let metric = GaugeVec::new(
            Opts::new("interrupts", "Per-IRQ per-CPU interrupt counters from /proc/interrupts"),
            &["irq", "cpu", "name"],
        )?;
        registry.register(Box::new(metric.clone()))?;
        Ok(Self { metric })
    }

    fn collect_once(&self) -> Result<()> {
        let s = fs::read_to_string("/proc/interrupts")?;
        let mut lines = s.lines();

        let header = match lines.next() {
            Some(h) => h,
            None => return Ok(()),
        };
        let ncpus = header.split_whitespace().filter(|t| t.starts_with("CPU")).count();

        let mut rows = 0usize;

        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut toks = line.split_whitespace();

            let irq_col = match toks.next() {
                Some(t) => t,
                None => continue,
            };
            let irq_id = irq_col.trim_end_matches(':');

            let mut cpu_counts: Vec<&str> = Vec::with_capacity(ncpus);
            for _ in 0..ncpus {
                if let Some(t) = toks.next() {
                    cpu_counts.push(t);
                } else {
                    break;
                }
            }

            let rest: Vec<&str> = toks.collect();
            let name = rest.last().cloned().unwrap_or("");

            for (cpu_idx, val_s) in cpu_counts.iter().enumerate() {
                if let Ok(v) = val_s.replace(',', "").parse::<u64>() {
                    self.metric
                        .with_label_values(&[irq_id, &cpu_idx.to_string(), name])
                        .set(v as f64);
                }
            }

            rows += 1;
        }

        debug!("interrupts: updated {rows} rows ({} CPUs)", ncpus);
        Ok(())
    }
}

impl Monitor for InterruptsMonitor {
    fn name(&self) -> &'static &str {
        &"interrupts"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
