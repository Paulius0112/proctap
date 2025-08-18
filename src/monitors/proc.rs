use std::{fs, path::PathBuf};

use log::debug;
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
    pub fn new(registry: &Registry, proc_name: String) -> anyhow::Result<Self> {
        let make_gauge = |name, help| -> anyhow::Result<GaugeVec> {
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
            sum_exec_runtime: make_gauge("proc_sum_exec_runtime", "sum_exec_runtime from /proc/<pid>/sched")?,
        })
    }

    fn read_comm(pid: &u32) -> Option<String> {
        let content = fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
        Some(content.trim().to_string())
    }

    fn read_sched(pid: u32) -> Option<ProcessSched> {
        let content = fs::read_to_string(format!("/proc/{pid}/sched")).ok()?;
        Self::parse_sched(&content)
    }

    fn parse_sched(content: &str) -> Option<ProcessSched> {
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

        Some(ProcessSched {
            nr_migrations: nr_migrations?,
            nr_switches: nr_switches?,
            nr_involuntary_switches: nr_involuntary_switches?,
            nr_voluntary_switches: nr_voluntary_switches?,
            sum_exec_runtime: sum_exec_runtime?,
        })
    }
}

impl Monitor for ProcessSchedMonitor {
    fn name(&self) -> &'static &str {
        &"sched"
    }

    fn collect(&mut self) -> anyhow::Result<()> {
        let mut matched = 0usize;

        for entry in fs::read_dir(PathBuf::from("/proc"))? {
            match entry {
                Ok(e) => {
                    if !e.file_type()?.is_dir() {
                        continue;
                    }

                    let Ok(pid) = e.file_name().to_string_lossy().parse::<u32>() else {
                        continue;
                    };

                    let Some(comm) = Self::read_comm(&pid) else {
                        continue;
                    };

                    if !comm.contains(&self.proc_name_filter) {
                        continue;
                    };

                    if let Some(s) = Self::read_sched(pid) {
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
                }
                Err(e) => {
                    eprintln!("Error reading dir: {}", e);
                }
            }
        }

        debug!("comm='{}' matched {} PIDs", self.proc_name_filter, matched);
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
