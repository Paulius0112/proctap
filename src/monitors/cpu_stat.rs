use anyhow::Result;
use log::{debug, info};
use prometheus::{Gauge, GaugeVec, Opts, Registry};
use std::fs;

use crate::monitor::Monitor;

pub struct CpuStatMonitor {
    cpu_time: GaugeVec,       // labels: [cpu, mode]
    context_switches: Gauge,
    boot_time: Gauge,
    processes_total: Gauge,
    procs_running: Gauge,
    procs_blocked: Gauge,
    interrupts_total: Gauge,
    softirq_total: Gauge,
}

impl CpuStatMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled CpuStat monitor");

        let cpu_time = GaugeVec::new(
            Opts::new(
                "cpu_stat_seconds",
                "Time spent in each CPU mode in seconds (from /proc/stat)",
            ),
            &["cpu", "mode"],
        )?;
        let context_switches = Gauge::new(
            "cpu_stat_context_switches_total",
            "Total number of context switches",
        )?;
        let boot_time = Gauge::new(
            "cpu_stat_boot_time_seconds",
            "System boot time in seconds since epoch",
        )?;
        let processes_total = Gauge::new(
            "cpu_stat_processes_total",
            "Total number of forks since boot",
        )?;
        let procs_running = Gauge::new(
            "cpu_stat_procs_running",
            "Number of processes currently running",
        )?;
        let procs_blocked = Gauge::new(
            "cpu_stat_procs_blocked",
            "Number of processes currently blocked for I/O",
        )?;
        let interrupts_total = Gauge::new(
            "cpu_stat_interrupts_total",
            "Total number of all interrupts serviced",
        )?;
        let softirq_total = Gauge::new(
            "cpu_stat_softirq_total",
            "Total number of all softirqs serviced",
        )?;

        registry.register(Box::new(cpu_time.clone()))?;
        registry.register(Box::new(context_switches.clone()))?;
        registry.register(Box::new(boot_time.clone()))?;
        registry.register(Box::new(processes_total.clone()))?;
        registry.register(Box::new(procs_running.clone()))?;
        registry.register(Box::new(procs_blocked.clone()))?;
        registry.register(Box::new(interrupts_total.clone()))?;
        registry.register(Box::new(softirq_total.clone()))?;

        Ok(Self {
            cpu_time,
            context_switches,
            boot_time,
            processes_total,
            procs_running,
            procs_blocked,
            interrupts_total,
            softirq_total,
        })
    }

    fn parse_cpu_line(&self, cpu_name: &str, values: &[&str]) {
        // Values are in USER_HZ (typically 100), convert to seconds
        const USER_HZ: f64 = 100.0;

        let modes = [
            "user", "nice", "system", "idle", "iowait", "irq", "softirq", "steal", "guest",
            "guest_nice",
        ];

        for (i, mode) in modes.iter().enumerate() {
            if let Some(val_str) = values.get(i) {
                if let Ok(val) = val_str.parse::<u64>() {
                    self.cpu_time
                        .with_label_values(&[cpu_name, mode])
                        .set(val as f64 / USER_HZ);
                }
            }
        }
    }

    fn collect_once(&self) -> Result<()> {
        let content = fs::read_to_string("/proc/stat")?;
        let mut cpu_count = 0usize;

        for line in content.lines() {
            if line.starts_with("cpu") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 1 {
                    let cpu_name = parts[0];
                    self.parse_cpu_line(cpu_name, &parts[1..]);
                    if cpu_name != "cpu" {
                        cpu_count += 1;
                    }
                }
            } else if let Some(rest) = line.strip_prefix("ctxt ") {
                if let Ok(v) = rest.trim().parse::<f64>() {
                    self.context_switches.set(v);
                }
            } else if let Some(rest) = line.strip_prefix("btime ") {
                if let Ok(v) = rest.trim().parse::<f64>() {
                    self.boot_time.set(v);
                }
            } else if let Some(rest) = line.strip_prefix("processes ") {
                if let Ok(v) = rest.trim().parse::<f64>() {
                    self.processes_total.set(v);
                }
            } else if let Some(rest) = line.strip_prefix("procs_running ") {
                if let Ok(v) = rest.trim().parse::<f64>() {
                    self.procs_running.set(v);
                }
            } else if let Some(rest) = line.strip_prefix("procs_blocked ") {
                if let Ok(v) = rest.trim().parse::<f64>() {
                    self.procs_blocked.set(v);
                }
            } else if let Some(rest) = line.strip_prefix("intr ") {
                // First value is total
                if let Some(total) = rest.split_whitespace().next() {
                    if let Ok(v) = total.parse::<f64>() {
                        self.interrupts_total.set(v);
                    }
                }
            } else if let Some(rest) = line.strip_prefix("softirq ") {
                // First value is total
                if let Some(total) = rest.split_whitespace().next() {
                    if let Ok(v) = total.parse::<f64>() {
                        self.softirq_total.set(v);
                    }
                }
            }
        }

        debug!("cpu_stat: updated {} CPUs", cpu_count);
        Ok(())
    }
}

impl Monitor for CpuStatMonitor {
    fn name(&self) -> &'static &str {
        &"cpu_stat"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
