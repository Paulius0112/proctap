use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::monitor::MonitorKind;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_interval")]
    pub interval: u64,
    #[serde(default)]
    pub monitors: Vec<String>,
    #[serde(default)]
    pub proc_prefixes: Vec<String>,
}

fn default_port() -> u16 {
    9000
}

fn default_bind() -> String {
    "0.0.0.0".to_string()
}

fn default_interval() -> u64 {
    5
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: Config =
            toml::from_str(&content).with_context(|| "Failed to parse config file")?;
        Ok(config)
    }

    pub fn parse_monitors(&self) -> Vec<MonitorKind> {
        self.monitors
            .iter()
            .filter_map(|s| match s.to_lowercase().as_str() {
                "sched" => Some(MonitorKind::Sched),
                "snmp" => Some(MonitorKind::Snmp),
                "netdev" => Some(MonitorKind::NetDev),
                "netdevqueues" => Some(MonitorKind::NetDevQueues),
                "diskstat" => Some(MonitorKind::DiskStat),
                "interrupts" => Some(MonitorKind::Interrupts),
                "memstat" => Some(MonitorKind::MemStat),
                "softirqs" => Some(MonitorKind::SoftIrqs),
                "softnetstat" => Some(MonitorKind::SoftNetStat),
                "ethtool" => Some(MonitorKind::Ethtool),
                "cpustat" => Some(MonitorKind::CpuStat),
                "loadavg" => Some(MonitorKind::LoadAvg),
                "cpufreq" => Some(MonitorKind::CpuFreq),
                "cpuidle" => Some(MonitorKind::CpuIdle),
                "schedstat" => Some(MonitorKind::SchedStat),
                "vmstat" => Some(MonitorKind::VmStat),
                "zoneinfo" => Some(MonitorKind::ZoneInfo),
                "buddyinfo" => Some(MonitorKind::BuddyInfo),
                "slabinfo" => Some(MonitorKind::SlabInfo),
                "procextended" => Some(MonitorKind::ProcExtended),
                "netstat" => Some(MonitorKind::NetStat),
                "sockstat" => Some(MonitorKind::SockStat),
                "netinfo" => Some(MonitorKind::NetInfo),
                "blkqueue" => Some(MonitorKind::BlkQueue),
                "blkinflight" => Some(MonitorKind::BlkInflight),
                "entropy" => Some(MonitorKind::Entropy),
                "fsstats" => Some(MonitorKind::FsStats),
                "uptime" => Some(MonitorKind::Uptime),
                "thermal" => Some(MonitorKind::Thermal),
                "cgroup" => Some(MonitorKind::Cgroup),
                _ => None,
            })
            .collect()
    }
}
