use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;

use crate::monitor::Monitor;

pub struct BuddyInfoMonitor {
    free_blocks: GaugeVec, // labels: [node, zone, order]
}

impl BuddyInfoMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled BuddyInfo monitor");

        let free_blocks = GaugeVec::new(
            Opts::new(
                "buddyinfo_free_blocks",
                "Number of free blocks at each order (2^order pages) per NUMA node and zone",
            ),
            &["node", "zone", "order"],
        )?;

        registry.register(Box::new(free_blocks.clone()))?;

        Ok(Self { free_blocks })
    }

    fn collect_once(&self) -> Result<()> {
        // Format: "Node 0, zone      DMA      1      1      1      0      2      1 ..."
        let content = fs::read_to_string("/proc/buddyinfo")?;
        let mut entry_count = 0usize;

        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("Node ") {
                continue;
            }

            // Parse "Node N, zone   ZoneName  <values...>"
            let parts: Vec<&str> = trimmed.splitn(2, ", zone").collect();
            if parts.len() != 2 {
                continue;
            }

            let node = parts[0]
                .strip_prefix("Node ")
                .unwrap_or("0")
                .trim()
                .to_string();

            let zone_and_values: Vec<&str> = parts[1].split_whitespace().collect();
            if zone_and_values.is_empty() {
                continue;
            }

            let zone = zone_and_values[0].to_string();

            // Remaining values are free block counts at each order
            for (order, count_str) in zone_and_values.iter().skip(1).enumerate() {
                if let Ok(count) = count_str.parse::<u64>() {
                    self.free_blocks
                        .with_label_values(&[&node, &zone, &order.to_string()])
                        .set(count as f64);
                }
            }

            entry_count += 1;
        }

        debug!("buddyinfo: updated {} entries", entry_count);
        Ok(())
    }
}

impl Monitor for BuddyInfoMonitor {
    fn name(&self) -> &'static &str {
        &"buddyinfo"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
