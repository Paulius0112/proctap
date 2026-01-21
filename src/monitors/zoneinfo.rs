use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;

use crate::monitor::Monitor;

pub struct ZoneInfoMonitor {
    pages: GaugeVec,      // labels: [node, zone, key]
    watermarks: GaugeVec, // labels: [node, zone, type]
}

impl ZoneInfoMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled ZoneInfo monitor");

        let pages = GaugeVec::new(
            Opts::new(
                "zoneinfo_pages",
                "Memory zone statistics in pages from /proc/zoneinfo",
            ),
            &["node", "zone", "key"],
        )?;
        let watermarks = GaugeVec::new(
            Opts::new(
                "zoneinfo_watermark_pages",
                "Memory zone watermark values in pages",
            ),
            &["node", "zone", "type"],
        )?;

        registry.register(Box::new(pages.clone()))?;
        registry.register(Box::new(watermarks.clone()))?;

        Ok(Self { pages, watermarks })
    }

    fn collect_once(&self) -> Result<()> {
        let content = fs::read_to_string("/proc/zoneinfo")?;
        let mut zone_count = 0usize;

        let mut current_node = String::new();
        let mut current_zone = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Node line: "Node 0, zone   Normal"
            if trimmed.starts_with("Node ") {
                if let Some((node_part, zone_part)) = trimmed.split_once(", zone") {
                    if let Some(node_num) = node_part.strip_prefix("Node ") {
                        current_node = node_num.trim().to_string();
                    }
                    current_zone = zone_part.trim().to_string();
                    zone_count += 1;
                }
                continue;
            }

            if current_node.is_empty() || current_zone.is_empty() {
                continue;
            }

            // Parse key-value lines
            if let Some((key, value)) = trimmed.split_once(' ') {
                let key = key.trim();
                let value = value.trim();

                if let Ok(v) = value.parse::<u64>() {
                    match key {
                        "min" | "low" | "high" => {
                            self.watermarks
                                .with_label_values(&[current_node.as_str(), current_zone.as_str(), key])
                                .set(v as f64);
                        }
                        "free" | "nr_free_pages" | "nr_zone_inactive_anon"
                        | "nr_zone_active_anon" | "nr_zone_inactive_file"
                        | "nr_zone_active_file" | "nr_zone_unevictable" | "nr_zone_write_pending"
                        | "nr_mlock" | "nr_bounce" | "nr_zspages" | "nr_free_cma" => {
                            self.pages
                                .with_label_values(&[current_node.as_str(), current_zone.as_str(), key])
                                .set(v as f64);
                        }
                        _ => {
                            // Include other numeric values
                            if key.starts_with("nr_") || key == "managed" || key == "present"
                                || key == "spanned" || key == "protection:"
                            {
                                continue; // Skip protection array and some meta fields
                            }
                        }
                    }
                }
            }
        }

        debug!("zoneinfo: updated {} zones", zone_count);
        Ok(())
    }
}

impl Monitor for ZoneInfoMonitor {
    fn name(&self) -> &'static &str {
        &"zoneinfo"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
