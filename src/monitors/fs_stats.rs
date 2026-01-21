use anyhow::Result;
use log::{debug, info};
use prometheus::{Gauge, Registry};
use std::fs;
use std::path::Path;

use crate::monitor::Monitor;

pub struct FsStatsMonitor {
    file_nr_allocated: Gauge,
    file_nr_free: Gauge,
    file_nr_max: Gauge,
    inode_nr_allocated: Gauge,
    inode_nr_free: Gauge,
    dentry_nr_allocated: Gauge,
    dentry_nr_unused: Gauge,
    dentry_age_limit: Gauge,
}

impl FsStatsMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled FsStats monitor");

        let file_nr_allocated = Gauge::new(
            "fs_file_nr_allocated",
            "Number of allocated file handles",
        )?;
        let file_nr_free = Gauge::new(
            "fs_file_nr_free",
            "Number of free file handles (always 0 in modern kernels)",
        )?;
        let file_nr_max = Gauge::new(
            "fs_file_nr_max",
            "Maximum number of file handles",
        )?;
        let inode_nr_allocated = Gauge::new(
            "fs_inode_nr_allocated",
            "Number of allocated inodes",
        )?;
        let inode_nr_free = Gauge::new(
            "fs_inode_nr_free",
            "Number of free inodes",
        )?;
        let dentry_nr_allocated = Gauge::new(
            "fs_dentry_nr_allocated",
            "Number of allocated directory entry cache entries",
        )?;
        let dentry_nr_unused = Gauge::new(
            "fs_dentry_nr_unused",
            "Number of unused directory entry cache entries",
        )?;
        let dentry_age_limit = Gauge::new(
            "fs_dentry_age_limit_seconds",
            "Age in seconds after which dcache entries can be reclaimed",
        )?;

        registry.register(Box::new(file_nr_allocated.clone()))?;
        registry.register(Box::new(file_nr_free.clone()))?;
        registry.register(Box::new(file_nr_max.clone()))?;
        registry.register(Box::new(inode_nr_allocated.clone()))?;
        registry.register(Box::new(inode_nr_free.clone()))?;
        registry.register(Box::new(dentry_nr_allocated.clone()))?;
        registry.register(Box::new(dentry_nr_unused.clone()))?;
        registry.register(Box::new(dentry_age_limit.clone()))?;

        Ok(Self {
            file_nr_allocated,
            file_nr_free,
            file_nr_max,
            inode_nr_allocated,
            inode_nr_free,
            dentry_nr_allocated,
            dentry_nr_unused,
            dentry_age_limit,
        })
    }

    fn parse_file_nr(content: &str, allocated: &Gauge, free: &Gauge, max: &Gauge) {
        // Format: "allocated\tfree\tmax"
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 3 {
            if let Ok(v) = parts[0].parse::<f64>() {
                allocated.set(v);
            }
            if let Ok(v) = parts[1].parse::<f64>() {
                free.set(v);
            }
            if let Ok(v) = parts[2].parse::<f64>() {
                max.set(v);
            }
        }
    }

    fn parse_inode_nr(content: &str, allocated: &Gauge, free: &Gauge) {
        // Format: "allocated\tfree"
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(v) = parts[0].parse::<f64>() {
                allocated.set(v);
            }
            if let Ok(v) = parts[1].parse::<f64>() {
                free.set(v);
            }
        }
    }

    fn parse_dentry(content: &str, allocated: &Gauge, unused: &Gauge, age: &Gauge) {
        // Format: "allocated unused age_limit want_pages"
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 3 {
            if let Ok(v) = parts[0].parse::<f64>() {
                allocated.set(v);
            }
            if let Ok(v) = parts[1].parse::<f64>() {
                unused.set(v);
            }
            if let Ok(v) = parts[2].parse::<f64>() {
                age.set(v);
            }
        }
    }

    fn collect_once(&self) -> Result<()> {
        let base = Path::new("/proc/sys/fs");

        if let Ok(content) = fs::read_to_string(base.join("file-nr")) {
            Self::parse_file_nr(
                &content,
                &self.file_nr_allocated,
                &self.file_nr_free,
                &self.file_nr_max,
            );
        }

        if let Ok(content) = fs::read_to_string(base.join("inode-nr")) {
            Self::parse_inode_nr(&content, &self.inode_nr_allocated, &self.inode_nr_free);
        }

        if let Ok(content) = fs::read_to_string(base.join("dentry-state")) {
            Self::parse_dentry(
                &content,
                &self.dentry_nr_allocated,
                &self.dentry_nr_unused,
                &self.dentry_age_limit,
            );
        }

        debug!("fs_stats: updated");
        Ok(())
    }
}

impl Monitor for FsStatsMonitor {
    fn name(&self) -> &'static &str {
        &"fs_stats"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
