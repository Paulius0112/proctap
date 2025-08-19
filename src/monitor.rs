use clap::ValueEnum;
#[derive(Debug, Clone, ValueEnum)]
pub enum MonitorKind {
    Sched,
    Snmp,
    NetDev,
    DiskStat,
}

pub trait Monitor {
    fn collect(&mut self) -> anyhow::Result<()>;
    fn name(&self) -> &'static &str;
}
