use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;
use std::path::PathBuf;

use crate::monitor::Monitor;

pub struct ThermalMonitor {
    root: PathBuf,
    temp: GaugeVec, // labels: [zone, type]
}

impl ThermalMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled Thermal monitor");

        let temp = GaugeVec::new(
            Opts::new(
                "thermal_zone_temp_celsius",
                "Temperature of thermal zone in Celsius",
            ),
            &["zone", "type"],
        )?;

        registry.register(Box::new(temp.clone()))?;

        Ok(Self {
            root: PathBuf::from("/sys/class/thermal"),
            temp,
        })
    }

    fn read_value(path: &std::path::Path) -> Option<f64> {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| s.trim().parse::<i64>().ok())
            .map(|millicelsius| millicelsius as f64 / 1000.0)
    }

    fn read_string(path: &std::path::Path) -> String {
        fs::read_to_string(path)
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    }

    fn collect_once(&self) -> Result<()> {
        let entries = match fs::read_dir(&self.root) {
            Ok(e) => e,
            Err(_) => return Ok(()), // thermal sysfs not available
        };

        let mut zone_count = 0usize;

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Match thermal_zone0, thermal_zone1, etc.
            if !name_str.starts_with("thermal_zone") {
                continue;
            }

            let zone_path = entry.path();
            let zone_label = name_str.to_string();
            let zone_type = Self::read_string(&zone_path.join("type"));

            if let Some(temp_celsius) = Self::read_value(&zone_path.join("temp")) {
                self.temp
                    .with_label_values(&[&zone_label, &zone_type])
                    .set(temp_celsius);
                zone_count += 1;
            }
        }

        debug!("thermal: updated {} zones", zone_count);
        Ok(())
    }
}

impl Monitor for ThermalMonitor {
    fn name(&self) -> &'static &str {
        &"thermal"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
