use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;

use crate::monitor::Monitor;

pub struct SchedStatMonitor {
    sched_count: GaugeVec,    // labels: [cpu]
    run_time_ns: GaugeVec,    // labels: [cpu]
    wait_time_ns: GaugeVec,   // labels: [cpu]
}

impl SchedStatMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled SchedStat monitor");

        let sched_count = GaugeVec::new(
            Opts::new(
                "schedstat_sched_count",
                "Number of times a task was scheduled on this CPU",
            ),
            &["cpu"],
        )?;
        let run_time_ns = GaugeVec::new(
            Opts::new(
                "schedstat_run_time_ns",
                "Time spent running tasks on this CPU in nanoseconds",
            ),
            &["cpu"],
        )?;
        let wait_time_ns = GaugeVec::new(
            Opts::new(
                "schedstat_wait_time_ns",
                "Time tasks spent waiting to run on this CPU in nanoseconds",
            ),
            &["cpu"],
        )?;

        registry.register(Box::new(sched_count.clone()))?;
        registry.register(Box::new(run_time_ns.clone()))?;
        registry.register(Box::new(wait_time_ns.clone()))?;

        Ok(Self {
            sched_count,
            run_time_ns,
            wait_time_ns,
        })
    }

    fn collect_once(&self) -> Result<()> {
        let content = fs::read_to_string("/proc/schedstat")?;
        let mut cpu_count = 0usize;

        for line in content.lines() {
            // Format: "cpu0 9 0 ... <more fields>"
            // Fields vary by kernel version, but first 3 after "cpuN" are consistent:
            // yld_count, sched_count (skip), run_time, wait_time
            // Actually the format is: cpuN yld_count sched_switch sched_count run_time wait_time ...
            // Let's parse the stable fields

            if !line.starts_with("cpu") {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 8 {
                continue;
            }

            let cpu_name = parts[0];

            // Version 15 format (most common):
            // cpu<N> <yld_count> <sched_switch> <sched_count> <sched_goidle> <ttwu_count> <ttwu_local> <rq_run_time> <rq_wait_time>
            // Indices: 0=cpu, 1=yld, 2=switch, 3=count, 4=goidle, 5=ttwu, 6=ttwu_local, 7=run_time, 8=wait_time

            if let Ok(v) = parts[3].parse::<f64>() {
                self.sched_count.with_label_values(&[cpu_name]).set(v);
            }
            if let Ok(v) = parts[7].parse::<f64>() {
                self.run_time_ns.with_label_values(&[cpu_name]).set(v);
            }
            if parts.len() > 8 {
                if let Ok(v) = parts[8].parse::<f64>() {
                    self.wait_time_ns.with_label_values(&[cpu_name]).set(v);
                }
            }

            cpu_count += 1;
        }

        debug!("schedstat: updated {} CPUs", cpu_count);
        Ok(())
    }
}

impl Monitor for SchedStatMonitor {
    fn name(&self) -> &'static &str {
        &"schedstat"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
