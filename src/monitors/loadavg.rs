use anyhow::Result;
use log::{debug, info};
use prometheus::{Gauge, Opts, Registry};
use std::fs;

use crate::monitor::Monitor;

pub struct LoadAvgMonitor {
    load_1m: Gauge,
    load_5m: Gauge,
    load_15m: Gauge,
    runnable_tasks: Gauge,
    total_tasks: Gauge,
    last_pid: Gauge,
}

impl LoadAvgMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled LoadAvg monitor");

        let load_1m = Gauge::new("loadavg_1m", "1-minute load average")?;
        let load_5m = Gauge::new("loadavg_5m", "5-minute load average")?;
        let load_15m = Gauge::new("loadavg_15m", "15-minute load average")?;
        let runnable_tasks = Gauge::with_opts(Opts::new(
            "loadavg_runnable_tasks",
            "Number of currently runnable kernel scheduling entities",
        ))?;
        let total_tasks = Gauge::with_opts(Opts::new(
            "loadavg_total_tasks",
            "Number of kernel scheduling entities that currently exist",
        ))?;
        let last_pid = Gauge::new("loadavg_last_pid", "PID of the most recently created process")?;

        registry.register(Box::new(load_1m.clone()))?;
        registry.register(Box::new(load_5m.clone()))?;
        registry.register(Box::new(load_15m.clone()))?;
        registry.register(Box::new(runnable_tasks.clone()))?;
        registry.register(Box::new(total_tasks.clone()))?;
        registry.register(Box::new(last_pid.clone()))?;

        Ok(Self {
            load_1m,
            load_5m,
            load_15m,
            runnable_tasks,
            total_tasks,
            last_pid,
        })
    }

    fn collect_once(&self) -> Result<()> {
        // Format: "0.00 0.01 0.05 1/234 5678"
        let content = fs::read_to_string("/proc/loadavg")?;
        let parts: Vec<&str> = content.split_whitespace().collect();

        if parts.len() >= 5 {
            if let Ok(v) = parts[0].parse::<f64>() {
                self.load_1m.set(v);
            }
            if let Ok(v) = parts[1].parse::<f64>() {
                self.load_5m.set(v);
            }
            if let Ok(v) = parts[2].parse::<f64>() {
                self.load_15m.set(v);
            }

            // Parse "runnable/total" format
            if let Some((runnable, total)) = parts[3].split_once('/') {
                if let Ok(r) = runnable.parse::<f64>() {
                    self.runnable_tasks.set(r);
                }
                if let Ok(t) = total.parse::<f64>() {
                    self.total_tasks.set(t);
                }
            }

            if let Ok(pid) = parts[4].parse::<f64>() {
                self.last_pid.set(pid);
            }
        }

        debug!("loadavg: updated");
        Ok(())
    }
}

impl Monitor for LoadAvgMonitor {
    fn name(&self) -> &'static &str {
        &"loadavg"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
