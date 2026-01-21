use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;
use std::path::PathBuf;

use crate::monitor::Monitor;
use crate::parsers::parse_proc_pid_stat;

pub struct ProcExtendedMonitor {
    proc_prefixes: Vec<String>,
    // From /proc/<pid>/status
    vm_rss_bytes: GaugeVec,
    vm_size_bytes: GaugeVec,
    threads: GaugeVec,
    voluntary_ctxt_switches: GaugeVec,
    nonvoluntary_ctxt_switches: GaugeVec,
    // From /proc/<pid>/io
    io_read_bytes: GaugeVec,
    io_write_bytes: GaugeVec,
    io_syscr: GaugeVec,
    io_syscw: GaugeVec,
    io_rchar: GaugeVec,
    io_wchar: GaugeVec,
    // From /proc/<pid>/stat
    utime: GaugeVec,
    stime: GaugeVec,
    minflt: GaugeVec,
    majflt: GaugeVec,
    num_threads: GaugeVec,
    starttime: GaugeVec,
    // From /proc/<pid>/schedstat
    sched_run_time_ns: GaugeVec,
    sched_wait_time_ns: GaugeVec,
    sched_timeslices: GaugeVec,
    // From /proc/<pid>/fd/
    fd_count: GaugeVec,
    // From /proc/<pid>/oom_score
    oom_score: GaugeVec,
    oom_score_adj: GaugeVec,
}

impl ProcExtendedMonitor {
    pub fn new(registry: &Registry, proc_prefixes: Vec<String>) -> Result<Self> {
        info!("Enabled ProcExtended monitor");

        let labels = &["proc", "pid"];

        // Status metrics
        let vm_rss_bytes = GaugeVec::new(
            Opts::new("proc_vm_rss_bytes", "Resident set size in bytes"),
            labels,
        )?;
        let vm_size_bytes = GaugeVec::new(
            Opts::new("proc_vm_size_bytes", "Virtual memory size in bytes"),
            labels,
        )?;
        let threads = GaugeVec::new(
            Opts::new("proc_threads", "Number of threads"),
            labels,
        )?;
        let voluntary_ctxt_switches = GaugeVec::new(
            Opts::new("proc_voluntary_ctxt_switches", "Voluntary context switches"),
            labels,
        )?;
        let nonvoluntary_ctxt_switches = GaugeVec::new(
            Opts::new("proc_nonvoluntary_ctxt_switches", "Non-voluntary context switches"),
            labels,
        )?;

        // IO metrics
        let io_read_bytes = GaugeVec::new(
            Opts::new("proc_io_read_bytes", "Bytes read from storage"),
            labels,
        )?;
        let io_write_bytes = GaugeVec::new(
            Opts::new("proc_io_write_bytes", "Bytes written to storage"),
            labels,
        )?;
        let io_syscr = GaugeVec::new(
            Opts::new("proc_io_syscr", "Read syscalls"),
            labels,
        )?;
        let io_syscw = GaugeVec::new(
            Opts::new("proc_io_syscw", "Write syscalls"),
            labels,
        )?;
        let io_rchar = GaugeVec::new(
            Opts::new("proc_io_rchar", "Characters read"),
            labels,
        )?;
        let io_wchar = GaugeVec::new(
            Opts::new("proc_io_wchar", "Characters written"),
            labels,
        )?;

        // Stat metrics
        let utime = GaugeVec::new(
            Opts::new("proc_utime_ticks", "User mode CPU time in ticks"),
            labels,
        )?;
        let stime = GaugeVec::new(
            Opts::new("proc_stime_ticks", "Kernel mode CPU time in ticks"),
            labels,
        )?;
        let minflt = GaugeVec::new(
            Opts::new("proc_minflt", "Minor page faults"),
            labels,
        )?;
        let majflt = GaugeVec::new(
            Opts::new("proc_majflt", "Major page faults"),
            labels,
        )?;
        let num_threads = GaugeVec::new(
            Opts::new("proc_num_threads", "Number of threads from stat"),
            labels,
        )?;
        let starttime = GaugeVec::new(
            Opts::new("proc_starttime_ticks", "Process start time in ticks since boot"),
            labels,
        )?;

        // Schedstat metrics
        let sched_run_time_ns = GaugeVec::new(
            Opts::new("proc_sched_run_time_ns", "Time spent running in nanoseconds"),
            labels,
        )?;
        let sched_wait_time_ns = GaugeVec::new(
            Opts::new("proc_sched_wait_time_ns", "Time spent waiting to run in nanoseconds"),
            labels,
        )?;
        let sched_timeslices = GaugeVec::new(
            Opts::new("proc_sched_timeslices", "Number of timeslices run"),
            labels,
        )?;

        // FD count
        let fd_count = GaugeVec::new(
            Opts::new("proc_fd_count", "Number of open file descriptors"),
            labels,
        )?;

        // OOM metrics
        let oom_score = GaugeVec::new(
            Opts::new("proc_oom_score", "OOM killer score (0-1000)"),
            labels,
        )?;
        let oom_score_adj = GaugeVec::new(
            Opts::new("proc_oom_score_adj", "OOM killer score adjustment (-1000 to 1000)"),
            labels,
        )?;

        // Register all metrics
        registry.register(Box::new(vm_rss_bytes.clone()))?;
        registry.register(Box::new(vm_size_bytes.clone()))?;
        registry.register(Box::new(threads.clone()))?;
        registry.register(Box::new(voluntary_ctxt_switches.clone()))?;
        registry.register(Box::new(nonvoluntary_ctxt_switches.clone()))?;
        registry.register(Box::new(io_read_bytes.clone()))?;
        registry.register(Box::new(io_write_bytes.clone()))?;
        registry.register(Box::new(io_syscr.clone()))?;
        registry.register(Box::new(io_syscw.clone()))?;
        registry.register(Box::new(io_rchar.clone()))?;
        registry.register(Box::new(io_wchar.clone()))?;
        registry.register(Box::new(utime.clone()))?;
        registry.register(Box::new(stime.clone()))?;
        registry.register(Box::new(minflt.clone()))?;
        registry.register(Box::new(majflt.clone()))?;
        registry.register(Box::new(num_threads.clone()))?;
        registry.register(Box::new(starttime.clone()))?;
        registry.register(Box::new(sched_run_time_ns.clone()))?;
        registry.register(Box::new(sched_wait_time_ns.clone()))?;
        registry.register(Box::new(sched_timeslices.clone()))?;
        registry.register(Box::new(fd_count.clone()))?;
        registry.register(Box::new(oom_score.clone()))?;
        registry.register(Box::new(oom_score_adj.clone()))?;

        Ok(Self {
            proc_prefixes,
            vm_rss_bytes,
            vm_size_bytes,
            threads,
            voluntary_ctxt_switches,
            nonvoluntary_ctxt_switches,
            io_read_bytes,
            io_write_bytes,
            io_syscr,
            io_syscw,
            io_rchar,
            io_wchar,
            utime,
            stime,
            minflt,
            majflt,
            num_threads,
            starttime,
            sched_run_time_ns,
            sched_wait_time_ns,
            sched_timeslices,
            fd_count,
            oom_score,
            oom_score_adj,
        })
    }

    fn read_comm(proc_path: &PathBuf) -> Option<String> {
        fs::read_to_string(proc_path.join("comm"))
            .ok()
            .map(|s| s.trim().to_string())
    }

    fn collect_status(&self, proc_path: &PathBuf, comm: &str, pid: &str) {
        if let Ok(content) = fs::read_to_string(proc_path.join("status")) {
            for line in content.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim();
                    let value = value.trim();

                    match key {
                        "VmRSS" => {
                            // Parse "1234 kB" format
                            if let Some(num_str) = value.split_whitespace().next() {
                                if let Ok(kb) = num_str.parse::<u64>() {
                                    self.vm_rss_bytes
                                        .with_label_values(&[comm, pid])
                                        .set((kb * 1024) as f64);
                                }
                            }
                        }
                        "VmSize" => {
                            if let Some(num_str) = value.split_whitespace().next() {
                                if let Ok(kb) = num_str.parse::<u64>() {
                                    self.vm_size_bytes
                                        .with_label_values(&[comm, pid])
                                        .set((kb * 1024) as f64);
                                }
                            }
                        }
                        "Threads" => {
                            if let Ok(v) = value.parse::<f64>() {
                                self.threads.with_label_values(&[comm, pid]).set(v);
                            }
                        }
                        "voluntary_ctxt_switches" => {
                            if let Ok(v) = value.parse::<f64>() {
                                self.voluntary_ctxt_switches
                                    .with_label_values(&[comm, pid])
                                    .set(v);
                            }
                        }
                        "nonvoluntary_ctxt_switches" => {
                            if let Ok(v) = value.parse::<f64>() {
                                self.nonvoluntary_ctxt_switches
                                    .with_label_values(&[comm, pid])
                                    .set(v);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn collect_io(&self, proc_path: &PathBuf, comm: &str, pid: &str) {
        if let Ok(content) = fs::read_to_string(proc_path.join("io")) {
            for line in content.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim();
                    let value = value.trim();

                    if let Ok(v) = value.parse::<f64>() {
                        match key {
                            "read_bytes" => {
                                self.io_read_bytes.with_label_values(&[comm, pid]).set(v);
                            }
                            "write_bytes" => {
                                self.io_write_bytes.with_label_values(&[comm, pid]).set(v);
                            }
                            "syscr" => {
                                self.io_syscr.with_label_values(&[comm, pid]).set(v);
                            }
                            "syscw" => {
                                self.io_syscw.with_label_values(&[comm, pid]).set(v);
                            }
                            "rchar" => {
                                self.io_rchar.with_label_values(&[comm, pid]).set(v);
                            }
                            "wchar" => {
                                self.io_wchar.with_label_values(&[comm, pid]).set(v);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn collect_stat(&self, proc_path: &PathBuf, comm: &str, pid: &str) {
        if let Ok(content) = fs::read_to_string(proc_path.join("stat")) {
            if let Some((_pid, _comm, _state, fields)) = parse_proc_pid_stat(&content) {
                // Fields after state (0-indexed from fields array):
                // 0=ppid, 1=pgrp, 2=session, 3=tty_nr, 4=tpgid, 5=flags
                // 6=minflt, 7=cminflt, 8=majflt, 9=cmajflt
                // 10=utime, 11=stime, 12=cutime, 13=cstime
                // 14=priority, 15=nice, 16=num_threads, 17=itrealvalue
                // 18=starttime

                if let Some(v) = fields.get(6).and_then(|s| s.parse::<f64>().ok()) {
                    self.minflt.with_label_values(&[comm, pid]).set(v);
                }
                if let Some(v) = fields.get(8).and_then(|s| s.parse::<f64>().ok()) {
                    self.majflt.with_label_values(&[comm, pid]).set(v);
                }
                if let Some(v) = fields.get(10).and_then(|s| s.parse::<f64>().ok()) {
                    self.utime.with_label_values(&[comm, pid]).set(v);
                }
                if let Some(v) = fields.get(11).and_then(|s| s.parse::<f64>().ok()) {
                    self.stime.with_label_values(&[comm, pid]).set(v);
                }
                if let Some(v) = fields.get(16).and_then(|s| s.parse::<f64>().ok()) {
                    self.num_threads.with_label_values(&[comm, pid]).set(v);
                }
                if let Some(v) = fields.get(18).and_then(|s| s.parse::<f64>().ok()) {
                    self.starttime.with_label_values(&[comm, pid]).set(v);
                }
            }
        }
    }

    fn collect_schedstat(&self, proc_path: &PathBuf, comm: &str, pid: &str) {
        if let Ok(content) = fs::read_to_string(proc_path.join("schedstat")) {
            // Format: "run_time wait_time timeslices"
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() >= 3 {
                if let Ok(v) = parts[0].parse::<f64>() {
                    self.sched_run_time_ns.with_label_values(&[comm, pid]).set(v);
                }
                if let Ok(v) = parts[1].parse::<f64>() {
                    self.sched_wait_time_ns.with_label_values(&[comm, pid]).set(v);
                }
                if let Ok(v) = parts[2].parse::<f64>() {
                    self.sched_timeslices.with_label_values(&[comm, pid]).set(v);
                }
            }
        }
    }

    fn collect_fd_count(&self, proc_path: &PathBuf, comm: &str, pid: &str) {
        if let Ok(entries) = fs::read_dir(proc_path.join("fd")) {
            let count = entries.count() as f64;
            self.fd_count.with_label_values(&[comm, pid]).set(count);
        }
    }

    fn collect_oom(&self, proc_path: &PathBuf, comm: &str, pid: &str) {
        if let Ok(content) = fs::read_to_string(proc_path.join("oom_score")) {
            if let Ok(v) = content.trim().parse::<f64>() {
                self.oom_score.with_label_values(&[comm, pid]).set(v);
            }
        }
        if let Ok(content) = fs::read_to_string(proc_path.join("oom_score_adj")) {
            if let Ok(v) = content.trim().parse::<f64>() {
                self.oom_score_adj.with_label_values(&[comm, pid]).set(v);
            }
        }
    }

    fn collect_once(&self) -> Result<()> {
        let proc_dir = PathBuf::from("/proc");
        let entries = fs::read_dir(&proc_dir)?;
        let mut proc_count = 0usize;

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Only process numeric directories (PIDs)
            if !name_str.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            let proc_path = entry.path();
            let pid_str = name_str.to_string();

            // Read comm to filter by prefix
            let comm = match Self::read_comm(&proc_path) {
                Some(c) => c,
                None => continue,
            };

            // Filter by prefix
            if !self.proc_prefixes.is_empty()
                && !self.proc_prefixes.iter().any(|p| comm.starts_with(p))
            {
                continue;
            }

            // Collect all metrics for this process
            self.collect_status(&proc_path, &comm, &pid_str);
            self.collect_io(&proc_path, &comm, &pid_str);
            self.collect_stat(&proc_path, &comm, &pid_str);
            self.collect_schedstat(&proc_path, &comm, &pid_str);
            self.collect_fd_count(&proc_path, &comm, &pid_str);
            self.collect_oom(&proc_path, &comm, &pid_str);

            proc_count += 1;
        }

        debug!("proc_extended: updated {} processes", proc_count);
        Ok(())
    }
}

impl Monitor for ProcExtendedMonitor {
    fn name(&self) -> &'static &str {
        &"proc_extended"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
