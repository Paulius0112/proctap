use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;

use crate::monitor::Monitor;

pub struct NetStatMonitor {
    tcp_ext: GaugeVec, // labels: [key]
    ip_ext: GaugeVec,  // labels: [key]
    mptcp: GaugeVec,   // labels: [key]
}

impl NetStatMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled NetStat monitor");

        let tcp_ext = GaugeVec::new(
            Opts::new(
                "netstat_tcp_ext",
                "Extended TCP statistics from /proc/net/netstat TcpExt",
            ),
            &["key"],
        )?;
        let ip_ext = GaugeVec::new(
            Opts::new(
                "netstat_ip_ext",
                "Extended IP statistics from /proc/net/netstat IpExt",
            ),
            &["key"],
        )?;
        let mptcp = GaugeVec::new(
            Opts::new(
                "netstat_mptcp",
                "MPTCP statistics from /proc/net/netstat MPTcpExt",
            ),
            &["key"],
        )?;

        registry.register(Box::new(tcp_ext.clone()))?;
        registry.register(Box::new(ip_ext.clone()))?;
        registry.register(Box::new(mptcp.clone()))?;

        Ok(Self {
            tcp_ext,
            ip_ext,
            mptcp,
        })
    }

    fn parse_protocol_lines(&self, header: &str, values: &str) {
        let header_parts: Vec<&str> = header.split_whitespace().collect();
        let value_parts: Vec<&str> = values.split_whitespace().collect();

        if header_parts.is_empty() || value_parts.is_empty() {
            return;
        }

        let protocol = header_parts[0].trim_end_matches(':');

        let gauge = match protocol {
            "TcpExt" => &self.tcp_ext,
            "IpExt" => &self.ip_ext,
            "MPTcpExt" => &self.mptcp,
            _ => return,
        };

        // Skip the protocol name in both arrays
        for (key, val) in header_parts.iter().skip(1).zip(value_parts.iter().skip(1)) {
            if let Ok(v) = val.parse::<u64>() {
                gauge.with_label_values(&[key]).set(v as f64);
            }
        }
    }

    fn collect_once(&self) -> Result<()> {
        // Format: paired lines of headers and values
        // TcpExt: SyncookiesSent SyncookiesRecv ...
        // TcpExt: 0 0 ...
        let content = fs::read_to_string("/proc/net/netstat")?;
        let lines: Vec<&str> = content.lines().collect();
        let mut seen = 0usize;

        let mut i = 0;
        while i + 1 < lines.len() {
            let header = lines[i];
            let values = lines[i + 1];

            // Check that both lines start with the same protocol
            if let Some(proto) = header.split(':').next() {
                if values.starts_with(proto) {
                    self.parse_protocol_lines(header, values);
                    seen += 1;
                }
            }
            i += 2;
        }

        debug!("netstat: updated {} protocol sections", seen);
        Ok(())
    }
}

impl Monitor for NetStatMonitor {
    fn name(&self) -> &'static &str {
        &"netstat"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
