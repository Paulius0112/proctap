// Existing monitors
pub mod diskstat;
pub mod ethtool;
pub mod interrupts;
pub mod memstat;
pub mod netdev_stat;
pub mod proc;
pub mod queues;
pub mod snmp;
pub mod softirqs;
pub mod softnet_stat;

// CPU monitors
pub mod cpu_stat;
pub mod cpufreq;
pub mod cpuidle;
pub mod loadavg;
pub mod schedstat;

// Memory monitors
pub mod buddyinfo;
pub mod slabinfo;
pub mod vmstat;
pub mod zoneinfo;

// Extended process monitor
pub mod proc_extended;

// Network monitors
pub mod netinfo;
pub mod netstat;
pub mod sockstat;

// Block I/O monitors
pub mod blkinflight;
pub mod blkqueue;

// System monitors
pub mod entropy;
pub mod fs_stats;
pub mod thermal;
pub mod uptime;

// cgroup monitor
pub mod cgroup;
