use clap::ValueEnum;
#[derive(Debug, Clone, ValueEnum)]
pub enum MonitorKind {
    Sched,
    Snmp,
    NetDev,
    DiskStat,
    Interrupts,
}

pub trait Monitor {
    fn collect(&mut self) -> anyhow::Result<()>;
    fn name(&self) -> &'static &str;
}
