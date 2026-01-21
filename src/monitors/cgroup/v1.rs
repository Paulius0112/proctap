use anyhow::Result;
use prometheus::GaugeVec;
use std::fs;
use std::path::Path;

/// Parse cgroup v1 CPU metrics from /sys/fs/cgroup/cpu,cpuacct/<cgroup>/
pub fn collect_cpu_v1(
    cgroup_path: &Path,
    cgroup_name: &str,
    usage_usec: &GaugeVec,
    user_usec: &GaugeVec,
    system_usec: &GaugeVec,
) -> Result<()> {
    // cpuacct.usage is in nanoseconds
    if let Ok(content) = fs::read_to_string(cgroup_path.join("cpuacct.usage")) {
        if let Ok(ns) = content.trim().parse::<u64>() {
            usage_usec
                .with_label_values(&[cgroup_name])
                .set((ns / 1000) as f64);
        }
    }

    // cpuacct.stat has user and system in USER_HZ (typically 100)
    if let Ok(content) = fs::read_to_string(cgroup_path.join("cpuacct.stat")) {
        const USER_HZ: f64 = 100.0;
        for line in content.lines() {
            if let Some((key, val)) = line.split_once(' ') {
                if let Ok(ticks) = val.parse::<u64>() {
                    let usec = (ticks as f64 / USER_HZ) * 1_000_000.0;
                    match key {
                        "user" => {
                            user_usec.with_label_values(&[cgroup_name]).set(usec);
                        }
                        "system" => {
                            system_usec.with_label_values(&[cgroup_name]).set(usec);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}

/// Parse cgroup v1 memory metrics from /sys/fs/cgroup/memory/<cgroup>/
pub fn collect_memory_v1(
    cgroup_path: &Path,
    cgroup_name: &str,
    current_bytes: &GaugeVec,
    max_bytes: &GaugeVec,
    swap_current_bytes: &GaugeVec,
) -> Result<()> {
    // memory.usage_in_bytes
    if let Ok(content) = fs::read_to_string(cgroup_path.join("memory.usage_in_bytes")) {
        if let Ok(bytes) = content.trim().parse::<f64>() {
            current_bytes.with_label_values(&[cgroup_name]).set(bytes);
        }
    }

    // memory.limit_in_bytes (may be very large for unlimited)
    if let Ok(content) = fs::read_to_string(cgroup_path.join("memory.limit_in_bytes")) {
        if let Ok(bytes) = content.trim().parse::<f64>() {
            // Cap at a reasonable max (e.g., 1 PB) to indicate unlimited
            if bytes < 1e18 {
                max_bytes.with_label_values(&[cgroup_name]).set(bytes);
            }
        }
    }

    // memory.memsw.usage_in_bytes (memory + swap)
    if let Ok(content) = fs::read_to_string(cgroup_path.join("memory.memsw.usage_in_bytes")) {
        if let Ok(bytes) = content.trim().parse::<f64>() {
            swap_current_bytes
                .with_label_values(&[cgroup_name])
                .set(bytes);
        }
    }

    Ok(())
}

/// Parse cgroup v1 blkio metrics from /sys/fs/cgroup/blkio/<cgroup>/
pub fn collect_io_v1(
    cgroup_path: &Path,
    cgroup_name: &str,
    io_read_bytes: &GaugeVec,
    io_write_bytes: &GaugeVec,
) -> Result<()> {
    // blkio.throttle.io_service_bytes format:
    // "major:minor Read bytes"
    // "major:minor Write bytes"
    // "Total bytes"
    if let Ok(content) = fs::read_to_string(cgroup_path.join("blkio.throttle.io_service_bytes")) {
        let mut total_read = 0u64;
        let mut total_write = 0u64;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let device = parts[0];
                let op = parts[1];
                if let Ok(bytes) = parts[2].parse::<u64>() {
                    // Skip "Total" aggregation lines
                    if device != "Total" {
                        match op {
                            "Read" => total_read += bytes,
                            "Write" => total_write += bytes,
                            _ => {}
                        }
                    }
                }
            }
        }

        io_read_bytes
            .with_label_values(&[cgroup_name, "total"])
            .set(total_read as f64);
        io_write_bytes
            .with_label_values(&[cgroup_name, "total"])
            .set(total_write as f64);
    }

    Ok(())
}

/// Parse cgroup v1 PIDs metrics from /sys/fs/cgroup/pids/<cgroup>/
pub fn collect_pids_v1(
    cgroup_path: &Path,
    cgroup_name: &str,
    pids_current: &GaugeVec,
    pids_max: &GaugeVec,
) -> Result<()> {
    // pids.current
    if let Ok(content) = fs::read_to_string(cgroup_path.join("pids.current")) {
        if let Ok(count) = content.trim().parse::<f64>() {
            pids_current.with_label_values(&[cgroup_name]).set(count);
        }
    }

    // pids.max ("max" means unlimited)
    if let Ok(content) = fs::read_to_string(cgroup_path.join("pids.max")) {
        let trimmed = content.trim();
        if trimmed != "max" {
            if let Ok(count) = trimmed.parse::<f64>() {
                pids_max.with_label_values(&[cgroup_name]).set(count);
            }
        }
    }

    Ok(())
}
