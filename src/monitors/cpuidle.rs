use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;
use std::path::PathBuf;

use crate::monitor::Monitor;

pub struct CpuIdleMonitor {
    root: PathBuf,
    usage: GaugeVec,   // labels: [cpu, state, name]
    time_us: GaugeVec, // labels: [cpu, state, name]
}

impl CpuIdleMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled CpuIdle monitor");

        let usage = GaugeVec::new(
            Opts::new(
                "cpuidle_usage_total",
                "Number of times this idle state was entered",
            ),
            &["cpu", "state", "name"],
        )?;
        let time_us = GaugeVec::new(
            Opts::new(
                "cpuidle_time_us",
                "Total time spent in this idle state in microseconds",
            ),
            &["cpu", "state", "name"],
        )?;

        registry.register(Box::new(usage.clone()))?;
        registry.register(Box::new(time_us.clone()))?;

        Ok(Self {
            root: PathBuf::from("/sys/devices/system/cpu"),
            usage,
            time_us,
        })
    }

    fn read_value(path: &std::path::Path) -> Option<f64> {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
    }

    fn read_string(path: &std::path::Path) -> String {
        fs::read_to_string(path)
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    }

    fn collect_once(&self) -> Result<()> {
        let entries = match fs::read_dir(&self.root) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        let mut state_count = 0usize;

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if !name_str.starts_with("cpu") {
                continue;
            }
            let cpu_num = &name_str[3..];
            if cpu_num.is_empty() || !cpu_num.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            let cpuidle_dir = entry.path().join("cpuidle");
            if !cpuidle_dir.exists() {
                continue;
            }

            let cpu_label = name_str.to_string();

            // Enumerate state0, state1, etc.
            if let Ok(state_entries) = fs::read_dir(&cpuidle_dir) {
                for state_entry in state_entries.flatten() {
                    let state_name = state_entry.file_name();
                    let state_str = state_name.to_string_lossy();

                    if !state_str.starts_with("state") {
                        continue;
                    }

                    let state_path = state_entry.path();
                    let state_label = state_str.to_string();
                    let idle_name = Self::read_string(&state_path.join("name"));

                    if let Some(v) = Self::read_value(&state_path.join("usage")) {
                        self.usage
                            .with_label_values(&[&cpu_label, &state_label, &idle_name])
                            .set(v);
                    }
                    if let Some(v) = Self::read_value(&state_path.join("time")) {
                        self.time_us
                            .with_label_values(&[&cpu_label, &state_label, &idle_name])
                            .set(v);
                    }

                    state_count += 1;
                }
            }
        }

        debug!("cpuidle: updated {} states", state_count);
        Ok(())
    }
}

impl Monitor for CpuIdleMonitor {
    fn name(&self) -> &'static &str {
        &"cpuidle"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
