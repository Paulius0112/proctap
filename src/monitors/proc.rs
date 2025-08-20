use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use log::{debug, error, warn};
use prometheus::{GaugeVec, Opts, Registry};

use crate::monitor::Monitor;

#[derive(Clone)]
pub struct ProcessSchedMonitor {
    proc_name_filter: String,
    nr_migrations: GaugeVec,
    nr_switches: GaugeVec,
    nr_involuntary_switches: GaugeVec,
    nr_voluntary_switches: GaugeVec,
    sum_exec_runtime: GaugeVec,
}

impl ProcessSchedMonitor {
    pub fn new(registry: &Registry, proc_name: String) -> Result<Self> {
        let make_gauge = |name: &str, help: &str| -> Result<GaugeVec> {
            let g = GaugeVec::new(Opts::new(name, help), &["proc", "pid"])?;
            registry.register(Box::new(g.clone()))?;
            Ok(g)
        };

        Ok(Self {
            proc_name_filter: proc_name,
            nr_migrations: make_gauge("proc_sched_nr_migrations", "se.nr_migrations from /proc/<pid>/sched")?,
            nr_switches: make_gauge("proc_sched_nr_switches", "nr_switches from /proc/<pid>/sched")?,
            nr_involuntary_switches: make_gauge(
                "proc_sched_nr_involuntary_switches",
                "nr_involuntary_switches from /proc/<pid>/sched",
            )?,
            nr_voluntary_switches: make_gauge(
                "proc_sched_nr_voluntary_switches",
                "nr_voluntary_switches from /proc/<pid>/sched",
            )?,
            sum_exec_runtime: make_gauge("proc_sum_exec_runtime", "se.sum_exec_runtime from /proc/<pid>/sched")?,
        })
    }

    fn read_comm(pid: &u32) -> Result<String> {
        let path = format!("/proc/{pid}/comm");
        let content = fs::read_to_string(&path).with_context(|| format!("reading {path}"))?;
        Ok(content.trim().to_string())
    }

    fn read_sched(pid: u32) -> Result<ProcessSched> {
        let path = format!("/proc/{pid}/sched");
        let content = fs::read_to_string(&path).with_context(|| format!("reading {path}"))?;
        Self::parse_sched(&content).with_context(|| format!("parsing {path}"))
    }

    fn parse_sched(content: &str) -> Result<ProcessSched> {
        let mut nr_migrations: Option<u64> = None;
        let mut nr_switches: Option<u64> = None;
        let mut nr_involuntary_switches: Option<u64> = None;
        let mut nr_voluntary_switches: Option<u64> = None;
        let mut sum_exec_runtime: Option<f64> = None;

        // skip header line
        for line in content.lines().skip(1) {
            let (k, v) = match line.split_once(':') {
                Some((k, v)) => (k.trim(), v.trim()),
                None => continue,
            };

            match k {
                "se.nr_migrations" => nr_migrations = v.parse().ok(),
                "nr_switches" => nr_switches = v.parse().ok(),
                "nr_involuntary_switches" => nr_involuntary_switches = v.parse().ok(),
                "nr_voluntary_switches" => nr_voluntary_switches = v.parse().ok(),
                "se.sum_exec_runtime" => sum_exec_runtime = v.parse().ok(),
                _ => {}
            }

            if nr_migrations.is_some()
                && nr_switches.is_some()
                && nr_involuntary_switches.is_some()
                && nr_voluntary_switches.is_some()
                && sum_exec_runtime.is_some()
            {
                break;
            }
        }

        Ok(ProcessSched {
            nr_migrations: nr_migrations.context("missing se.nr_migrations")?,
            nr_switches: nr_switches.context("missing nr_switches")?,
            nr_involuntary_switches: nr_involuntary_switches.context("missing nr_involuntary_switches")?,
            nr_voluntary_switches: nr_voluntary_switches.context("missing nr_voluntary_switches")?,
            sum_exec_runtime: sum_exec_runtime.context("missing se.sum_exec_runtime")?,
        })
    }
}

impl Monitor for ProcessSchedMonitor {
    fn name(&self) -> &'static &str {
        &"sched"
    }

    fn collect(&mut self) -> Result<()> {
        let mut matched = 0usize;

        let entries = fs::read_dir(PathBuf::from("/proc"))
            .with_context(|| "reading /proc")
            .map_err(|e| {
                error!("sched: {e:#}");
                e
            })?;

        for entry_res in entries {
            let entry = entry_res.map_err(|e| {
                error!("sched: iterating /proc: {e:#}");
                e
            })?;

            let ft = entry.file_type().map_err(|e| {
                error!("sched: reading file_type for {:?}: {e:#}", entry.path());
                e
            })?;
            if !ft.is_dir() {
                continue;
            }

            // Only numeric PIDs
            let pid: u32 = match entry.file_name().to_string_lossy().parse::<u32>() {
                Ok(p) => p,
                Err(_) => continue,
            };

            let comm = Self::read_comm(&pid).map_err(|e| {
                error!("sched: reading /proc/{pid}/comm: {e:#}");
                e
            })?;

            if !comm.starts_with(&self.proc_name_filter) {
                continue;
            }

            let s = Self::read_sched(pid).map_err(|e| {
                error!("sched: reading/parsing /proc/{pid}/sched (comm={comm}): {e:#}");
                e
            })?;

            matched += 1;
            let pid_s = pid.to_string();
            let labels = &[comm.as_str(), pid_s.as_str()];

            self.nr_migrations.with_label_values(labels).set(s.nr_migrations as f64);
            self.nr_switches.with_label_values(labels).set(s.nr_switches as f64);
            self.nr_involuntary_switches
                .with_label_values(labels)
                .set(s.nr_involuntary_switches as f64);
            self.nr_voluntary_switches
                .with_label_values(labels)
                .set(s.nr_voluntary_switches as f64);
            self.sum_exec_runtime.with_label_values(labels).set(s.sum_exec_runtime);
        }

        if matched == 0 {
            warn!("sched: no processes with prefix '{}' found", self.proc_name_filter);
        } else {
            debug!("comm prefix '{}' matched {} PIDs", self.proc_name_filter, matched);
        }

        Ok(())
    }
}

#[derive(Debug)]
struct ProcessSched {
    nr_migrations: u64,
    nr_switches: u64,
    nr_involuntary_switches: u64,
    nr_voluntary_switches: u64,
    sum_exec_runtime: f64,
}
