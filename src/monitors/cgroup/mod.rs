use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;
use std::path::PathBuf;

use crate::monitor::Monitor;

mod v1;
mod v2;

use v1 as cgroup_v1;
use v2 as cgroup_v2;

#[derive(Debug, Clone, Copy, PartialEq)]
enum CgroupVersion {
    V1,
    V2,
    Unknown,
}

pub struct CgroupMonitor {
    version: CgroupVersion,
    root: PathBuf,
    // CPU metrics
    cpu_usage_usec: GaugeVec,
    cpu_user_usec: GaugeVec,
    cpu_system_usec: GaugeVec,
    // Memory metrics
    memory_current_bytes: GaugeVec,
    memory_max_bytes: GaugeVec,
    memory_swap_current_bytes: GaugeVec,
    // I/O metrics (labels: cgroup, device)
    io_read_bytes: GaugeVec,
    io_write_bytes: GaugeVec,
    // PIDs metrics
    pids_current: GaugeVec,
    pids_max: GaugeVec,
}

impl CgroupMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled Cgroup monitor");

        // Detect cgroup version
        let (version, root) = Self::detect_cgroup_version();
        info!("Detected cgroup version: {:?} at {:?}", version, root);

        let cpu_usage_usec = GaugeVec::new(
            Opts::new("cgroup_cpu_usage_usec", "Total CPU usage in microseconds"),
            &["cgroup"],
        )?;
        let cpu_user_usec = GaugeVec::new(
            Opts::new("cgroup_cpu_user_usec", "User CPU usage in microseconds"),
            &["cgroup"],
        )?;
        let cpu_system_usec = GaugeVec::new(
            Opts::new(
                "cgroup_cpu_system_usec",
                "System CPU usage in microseconds",
            ),
            &["cgroup"],
        )?;
        let memory_current_bytes = GaugeVec::new(
            Opts::new(
                "cgroup_memory_current_bytes",
                "Current memory usage in bytes",
            ),
            &["cgroup"],
        )?;
        let memory_max_bytes = GaugeVec::new(
            Opts::new(
                "cgroup_memory_max_bytes",
                "Memory limit in bytes",
            ),
            &["cgroup"],
        )?;
        let memory_swap_current_bytes = GaugeVec::new(
            Opts::new(
                "cgroup_memory_swap_current_bytes",
                "Current swap usage in bytes",
            ),
            &["cgroup"],
        )?;
        let io_read_bytes = GaugeVec::new(
            Opts::new("cgroup_io_read_bytes", "Bytes read from block devices"),
            &["cgroup", "device"],
        )?;
        let io_write_bytes = GaugeVec::new(
            Opts::new("cgroup_io_write_bytes", "Bytes written to block devices"),
            &["cgroup", "device"],
        )?;
        let pids_current = GaugeVec::new(
            Opts::new("cgroup_pids_current", "Current number of PIDs"),
            &["cgroup"],
        )?;
        let pids_max = GaugeVec::new(
            Opts::new("cgroup_pids_max", "Maximum number of PIDs"),
            &["cgroup"],
        )?;

        registry.register(Box::new(cpu_usage_usec.clone()))?;
        registry.register(Box::new(cpu_user_usec.clone()))?;
        registry.register(Box::new(cpu_system_usec.clone()))?;
        registry.register(Box::new(memory_current_bytes.clone()))?;
        registry.register(Box::new(memory_max_bytes.clone()))?;
        registry.register(Box::new(memory_swap_current_bytes.clone()))?;
        registry.register(Box::new(io_read_bytes.clone()))?;
        registry.register(Box::new(io_write_bytes.clone()))?;
        registry.register(Box::new(pids_current.clone()))?;
        registry.register(Box::new(pids_max.clone()))?;

        Ok(Self {
            version,
            root,
            cpu_usage_usec,
            cpu_user_usec,
            cpu_system_usec,
            memory_current_bytes,
            memory_max_bytes,
            memory_swap_current_bytes,
            io_read_bytes,
            io_write_bytes,
            pids_current,
            pids_max,
        })
    }

    fn detect_cgroup_version() -> (CgroupVersion, PathBuf) {
        // Check for cgroup v2 unified hierarchy
        let v2_path = PathBuf::from("/sys/fs/cgroup");
        if v2_path.join("cgroup.controllers").exists() {
            return (CgroupVersion::V2, v2_path);
        }

        // Check for cgroup v1 hierarchy
        let v1_path = PathBuf::from("/sys/fs/cgroup");
        if v1_path.join("cpu").exists() || v1_path.join("memory").exists() {
            return (CgroupVersion::V1, v1_path);
        }

        (CgroupVersion::Unknown, v2_path)
    }

    fn collect_v2(&self) -> Result<()> {
        // In cgroup v2, we enumerate subdirectories under /sys/fs/cgroup
        self.enumerate_cgroups_v2(&self.root, "")?;
        Ok(())
    }

    fn enumerate_cgroups_v2(&self, path: &PathBuf, prefix: &str) -> Result<()> {
        let entries = match fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        for entry in entries.flatten() {
            let entry_path = entry.path();
            if !entry_path.is_dir() {
                continue;
            }

            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Skip special directories
            if name_str.starts_with('.') {
                continue;
            }

            let cgroup_name = if prefix.is_empty() {
                name_str.to_string()
            } else {
                format!("{}/{}", prefix, name_str)
            };

            // Collect metrics for this cgroup
            cgroup_v2::collect_cpu_v2(
                &entry_path,
                &cgroup_name,
                &self.cpu_usage_usec,
                &self.cpu_user_usec,
                &self.cpu_system_usec,
            )?;
            cgroup_v2::collect_memory_v2(
                &entry_path,
                &cgroup_name,
                &self.memory_current_bytes,
                &self.memory_max_bytes,
                &self.memory_swap_current_bytes,
            )?;
            cgroup_v2::collect_io_v2(
                &entry_path,
                &cgroup_name,
                &self.io_read_bytes,
                &self.io_write_bytes,
            )?;
            cgroup_v2::collect_pids_v2(
                &entry_path,
                &cgroup_name,
                &self.pids_current,
                &self.pids_max,
            )?;

            // Recurse into subdirectories (but limit depth)
            if prefix.matches('/').count() < 3 {
                self.enumerate_cgroups_v2(&entry_path, &cgroup_name)?;
            }
        }

        Ok(())
    }

    fn collect_v1(&self) -> Result<()> {
        // In cgroup v1, different controllers are in different hierarchies
        // We need to enumerate each controller separately

        // CPU controller
        let cpu_path = self.root.join("cpu,cpuacct");
        if cpu_path.exists() {
            self.enumerate_cgroups_v1(&cpu_path, "", "cpu")?;
        } else {
            let cpu_path = self.root.join("cpu");
            if cpu_path.exists() {
                self.enumerate_cgroups_v1(&cpu_path, "", "cpu")?;
            }
        }

        // Memory controller
        let memory_path = self.root.join("memory");
        if memory_path.exists() {
            self.enumerate_cgroups_v1(&memory_path, "", "memory")?;
        }

        // blkio controller
        let blkio_path = self.root.join("blkio");
        if blkio_path.exists() {
            self.enumerate_cgroups_v1(&blkio_path, "", "blkio")?;
        }

        // PIDs controller
        let pids_path = self.root.join("pids");
        if pids_path.exists() {
            self.enumerate_cgroups_v1(&pids_path, "", "pids")?;
        }

        Ok(())
    }

    fn enumerate_cgroups_v1(
        &self,
        path: &PathBuf,
        prefix: &str,
        controller: &str,
    ) -> Result<()> {
        let entries = match fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        for entry in entries.flatten() {
            let entry_path = entry.path();
            if !entry_path.is_dir() {
                continue;
            }

            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Skip special directories
            if name_str.starts_with('.') {
                continue;
            }

            let cgroup_name = if prefix.is_empty() {
                name_str.to_string()
            } else {
                format!("{}/{}", prefix, name_str)
            };

            // Collect metrics based on controller type
            match controller {
                "cpu" => {
                    cgroup_v1::collect_cpu_v1(
                        &entry_path,
                        &cgroup_name,
                        &self.cpu_usage_usec,
                        &self.cpu_user_usec,
                        &self.cpu_system_usec,
                    )?;
                }
                "memory" => {
                    cgroup_v1::collect_memory_v1(
                        &entry_path,
                        &cgroup_name,
                        &self.memory_current_bytes,
                        &self.memory_max_bytes,
                        &self.memory_swap_current_bytes,
                    )?;
                }
                "blkio" => {
                    cgroup_v1::collect_io_v1(
                        &entry_path,
                        &cgroup_name,
                        &self.io_read_bytes,
                        &self.io_write_bytes,
                    )?;
                }
                "pids" => {
                    cgroup_v1::collect_pids_v1(
                        &entry_path,
                        &cgroup_name,
                        &self.pids_current,
                        &self.pids_max,
                    )?;
                }
                _ => {}
            }

            // Recurse into subdirectories (but limit depth)
            if prefix.matches('/').count() < 3 {
                self.enumerate_cgroups_v1(&entry_path, &cgroup_name, controller)?;
            }
        }

        Ok(())
    }

    fn collect_once(&self) -> Result<()> {
        match self.version {
            CgroupVersion::V2 => self.collect_v2(),
            CgroupVersion::V1 => self.collect_v1(),
            CgroupVersion::Unknown => {
                debug!("cgroup: unknown version, skipping");
                Ok(())
            }
        }
    }
}

impl Monitor for CgroupMonitor {
    fn name(&self) -> &'static &str {
        &"cgroup"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
