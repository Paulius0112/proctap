use std::{fs, path::PathBuf};

use anyhow::{Context, Ok};
use prometheus::{GaugeVec, Opts, Registry};

use crate::monitor::Monitor;

pub struct SNMPMonitor {
    path: PathBuf,
    udp: GaugeVec,
    tcp: GaugeVec,
}

impl SNMPMonitor {
    pub fn new(registry: &Registry) -> anyhow::Result<Self> {
        let tcp = GaugeVec::new(Opts::new("snmp_tcp", "TCP Stats from /proc/net/snmp"), &["key"])?;
        registry.register(Box::new(tcp.clone()))?;

        let udp = GaugeVec::new(Opts::new("snmp_udp", "UDP Stats from /proc/net/snmp"), &["key"])?;
        registry.register(Box::new(udp.clone()))?;

        Ok(Self {
            path: PathBuf::from("/proc/net/snmp"),
            tcp,
            udp,
        })
    }

    fn parse_snmp_pairs(&self) -> anyhow::Result<Vec<(String, String, f64)>> {
        let content = fs::read_to_string(&self.path).unwrap();

        let mut out = Vec::new();
        let mut lines = content.lines();

        while let Some(hdr) = lines.next() {
            let Some(vals) = lines.next() else { break };

            let (proto_h, keys_str) = hdr.split_once(":").context("bad header line in /proc/net/snmp")?;
            let (proto_v, vals_str) = vals.split_once(":").context("bad value line in /proc/net/snmp")?;

            let proto = proto_h.trim();
            if proto != proto_v.trim() {
                continue;
            }

            if proto != "Tcp" && proto != "Udp" {
                continue;
            }

            let keys: Vec<&str> = keys_str.split_whitespace().collect();
            let vals: Vec<&str> = vals_str.split_whitespace().collect();
            if keys.len() != vals.len() {
                continue;
            }

            for (k, v) in keys.into_iter().zip(vals.into_iter()) {
                let n = v.parse::<f64>()?;
                out.push((proto.to_string(), k.to_string(), n));
            }
        }

        Ok(out)
    }
}

impl Monitor for SNMPMonitor {
    fn name(&self) -> &'static &str {
        &"snmp"
    }

    fn collect(&mut self) -> anyhow::Result<()> {
        for (proto, key, val) in self.parse_snmp_pairs()? {
            match proto.as_str() {
                // Replace with enum
                "Tcp" => {
                    self.tcp.with_label_values(&[key.to_string()]).set(val);
                }
                "Udp" => {
                    self.udp.with_label_values(&[key.to_string()]).set(val);
                }
                _ => {}
            }
        }

        Ok(())
    }
}
