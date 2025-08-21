use clap::ValueEnum;
#[derive(Debug, Clone, ValueEnum)]
pub enum MonitorKind {
    Sched,
    Snmp,
    NetDev,
    NetDevQueues,
    DiskStat,
    Interrupts,
    MemStat,
    SoftIrqs,
}

#[allow(dead_code)]
pub trait Monitor {
    fn collect(&mut self) -> anyhow::Result<()>;
    fn name(&self) -> &'static &str;
}
