use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;

use crate::monitor::Monitor;

pub struct SockStatMonitor {
    sockets: GaugeVec, // labels: [protocol, key]
}

impl SockStatMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled SockStat monitor");

        let sockets = GaugeVec::new(
            Opts::new(
                "sockstat",
                "Socket statistics from /proc/net/sockstat and sockstat6",
            ),
            &["protocol", "key"],
        )?;

        registry.register(Box::new(sockets.clone()))?;

        Ok(Self { sockets })
    }

    fn parse_sockstat_content(&self, content: &str, suffix: &str) {
        // Format: "protocol: key1 val1 key2 val2 ..."
        // e.g., "TCP: inuse 123 orphan 0 tw 456 alloc 789 mem 1234"
        for line in content.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() != 2 {
                continue;
            }

            let protocol = format!("{}{}", parts[0].trim(), suffix);
            let kv_parts: Vec<&str> = parts[1].split_whitespace().collect();

            // Parse key-value pairs
            let mut i = 0;
            while i + 1 < kv_parts.len() {
                let key = kv_parts[i];
                if let Ok(val) = kv_parts[i + 1].parse::<u64>() {
                    self.sockets
                        .with_label_values(&[protocol.as_str(), key])
                        .set(val as f64);
                }
                i += 2;
            }
        }
    }

    fn collect_once(&self) -> Result<()> {
        // Parse IPv4 sockstat
        if let Ok(content) = fs::read_to_string("/proc/net/sockstat") {
            self.parse_sockstat_content(&content, "");
        }

        // Parse IPv6 sockstat
        if let Ok(content) = fs::read_to_string("/proc/net/sockstat6") {
            self.parse_sockstat_content(&content, "6");
        }

        debug!("sockstat: updated");
        Ok(())
    }
}

impl Monitor for SockStatMonitor {
    fn name(&self) -> &'static &str {
        &"sockstat"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
