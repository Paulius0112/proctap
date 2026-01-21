use clap::ValueEnum;

#[derive(Debug, Clone, ValueEnum)]
pub enum MonitorKind {
    // Existing monitors
    Sched,
    Snmp,
    NetDev,
    NetDevQueues,
    DiskStat,
    Interrupts,
    MemStat,
    SoftIrqs,
    SoftNetStat,
    Ethtool,
    // CPU monitors
    CpuStat,
    LoadAvg,
    CpuFreq,
    CpuIdle,
    SchedStat,
    // Memory monitors
    VmStat,
    ZoneInfo,
    BuddyInfo,
    SlabInfo,
    // Process monitors
    ProcExtended,
    // Network monitors
    NetStat,
    SockStat,
    NetInfo,
    // Block I/O monitors
    BlkQueue,
    BlkInflight,
    // System monitors
    Entropy,
    FsStats,
    Uptime,
    Thermal,
    // cgroup monitor
    Cgroup,
}

#[allow(dead_code)]
pub trait Monitor {
    fn collect(&mut self) -> anyhow::Result<()>;
    fn name(&self) -> &'static &str;
}
