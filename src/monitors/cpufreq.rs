use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;
use std::path::PathBuf;

use crate::monitor::Monitor;

pub struct CpuFreqMonitor {
    root: PathBuf,
    current_freq: GaugeVec, // labels: [cpu]
    min_freq: GaugeVec,     // labels: [cpu]
    max_freq: GaugeVec,     // labels: [cpu]
    scaling_cur_freq: GaugeVec,
    scaling_min_freq: GaugeVec,
    scaling_max_freq: GaugeVec,
}

impl CpuFreqMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled CpuFreq monitor");

        let current_freq = GaugeVec::new(
            Opts::new(
                "cpufreq_current_hz",
                "Current CPU frequency in Hz (hardware reported)",
            ),
            &["cpu"],
        )?;
        let min_freq = GaugeVec::new(
            Opts::new(
                "cpufreq_min_hz",
                "Minimum CPU frequency in Hz (hardware limit)",
            ),
            &["cpu"],
        )?;
        let max_freq = GaugeVec::new(
            Opts::new(
                "cpufreq_max_hz",
                "Maximum CPU frequency in Hz (hardware limit)",
            ),
            &["cpu"],
        )?;
        let scaling_cur_freq = GaugeVec::new(
            Opts::new(
                "cpufreq_scaling_current_hz",
                "Current CPU frequency in Hz (governor setting)",
            ),
            &["cpu"],
        )?;
        let scaling_min_freq = GaugeVec::new(
            Opts::new(
                "cpufreq_scaling_min_hz",
                "Minimum scaling CPU frequency in Hz",
            ),
            &["cpu"],
        )?;
        let scaling_max_freq = GaugeVec::new(
            Opts::new(
                "cpufreq_scaling_max_hz",
                "Maximum scaling CPU frequency in Hz",
            ),
            &["cpu"],
        )?;

        registry.register(Box::new(current_freq.clone()))?;
        registry.register(Box::new(min_freq.clone()))?;
        registry.register(Box::new(max_freq.clone()))?;
        registry.register(Box::new(scaling_cur_freq.clone()))?;
        registry.register(Box::new(scaling_min_freq.clone()))?;
        registry.register(Box::new(scaling_max_freq.clone()))?;

        Ok(Self {
            root: PathBuf::from("/sys/devices/system/cpu"),
            current_freq,
            min_freq,
            max_freq,
            scaling_cur_freq,
            scaling_min_freq,
            scaling_max_freq,
        })
    }

    fn read_freq(path: &std::path::Path) -> Option<f64> {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|khz| khz as f64 * 1000.0) // kHz to Hz
    }

    fn collect_once(&self) -> Result<()> {
        let entries = match fs::read_dir(&self.root) {
            Ok(e) => e,
            Err(_) => return Ok(()), // sysfs not available (e.g., container)
        };

        let mut cpu_count = 0usize;

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Match cpu0, cpu1, etc.
            if !name_str.starts_with("cpu") {
                continue;
            }
            let cpu_num = &name_str[3..];
            if cpu_num.is_empty() || !cpu_num.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            let cpufreq_dir = entry.path().join("cpufreq");
            if !cpufreq_dir.exists() {
                continue;
            }

            let cpu_label = name_str.to_string();

            // Hardware frequencies
            if let Some(v) = Self::read_freq(&cpufreq_dir.join("cpuinfo_cur_freq")) {
                self.current_freq.with_label_values(&[&cpu_label]).set(v);
            }
            if let Some(v) = Self::read_freq(&cpufreq_dir.join("cpuinfo_min_freq")) {
                self.min_freq.with_label_values(&[&cpu_label]).set(v);
            }
            if let Some(v) = Self::read_freq(&cpufreq_dir.join("cpuinfo_max_freq")) {
                self.max_freq.with_label_values(&[&cpu_label]).set(v);
            }

            // Scaling frequencies (governor-controlled)
            if let Some(v) = Self::read_freq(&cpufreq_dir.join("scaling_cur_freq")) {
                self.scaling_cur_freq
                    .with_label_values(&[&cpu_label])
                    .set(v);
            }
            if let Some(v) = Self::read_freq(&cpufreq_dir.join("scaling_min_freq")) {
                self.scaling_min_freq
                    .with_label_values(&[&cpu_label])
                    .set(v);
            }
            if let Some(v) = Self::read_freq(&cpufreq_dir.join("scaling_max_freq")) {
                self.scaling_max_freq
                    .with_label_values(&[&cpu_label])
                    .set(v);
            }

            cpu_count += 1;
        }

        debug!("cpufreq: updated {} CPUs", cpu_count);
        Ok(())
    }
}

impl Monitor for CpuFreqMonitor {
    fn name(&self) -> &'static &str {
        &"cpufreq"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
