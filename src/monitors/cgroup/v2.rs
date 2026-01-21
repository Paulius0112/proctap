use anyhow::Result;
use prometheus::GaugeVec;
use std::fs;
use std::path::Path;

/// Parse cgroup v2 CPU metrics from cpu.stat
pub fn collect_cpu_v2(
    cgroup_path: &Path,
    cgroup_name: &str,
    usage_usec: &GaugeVec,
    user_usec: &GaugeVec,
    system_usec: &GaugeVec,
) -> Result<()> {
    // cpu.stat format:
    // usage_usec 12345
    // user_usec 6789
    // system_usec 5432
    if let Ok(content) = fs::read_to_string(cgroup_path.join("cpu.stat")) {
        for line in content.lines() {
            if let Some((key, val)) = line.split_once(' ') {
                if let Ok(v) = val.parse::<f64>() {
                    match key {
                        "usage_usec" => {
                            usage_usec.with_label_values(&[cgroup_name]).set(v);
                        }
                        "user_usec" => {
                            user_usec.with_label_values(&[cgroup_name]).set(v);
                        }
                        "system_usec" => {
                            system_usec.with_label_values(&[cgroup_name]).set(v);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}

/// Parse cgroup v2 memory metrics
pub fn collect_memory_v2(
    cgroup_path: &Path,
    cgroup_name: &str,
    current_bytes: &GaugeVec,
    max_bytes: &GaugeVec,
    swap_current_bytes: &GaugeVec,
) -> Result<()> {
    // memory.current
    if let Ok(content) = fs::read_to_string(cgroup_path.join("memory.current")) {
        if let Ok(bytes) = content.trim().parse::<f64>() {
            current_bytes.with_label_values(&[cgroup_name]).set(bytes);
        }
    }

    // memory.max ("max" means unlimited)
    if let Ok(content) = fs::read_to_string(cgroup_path.join("memory.max")) {
        let trimmed = content.trim();
        if trimmed != "max" {
            if let Ok(bytes) = trimmed.parse::<f64>() {
                max_bytes.with_label_values(&[cgroup_name]).set(bytes);
            }
        }
    }

    // memory.swap.current
    if let Ok(content) = fs::read_to_string(cgroup_path.join("memory.swap.current")) {
        if let Ok(bytes) = content.trim().parse::<f64>() {
            swap_current_bytes
                .with_label_values(&[cgroup_name])
                .set(bytes);
        }
    }

    Ok(())
}

/// Parse cgroup v2 I/O metrics from io.stat
pub fn collect_io_v2(
    cgroup_path: &Path,
    cgroup_name: &str,
    io_read_bytes: &GaugeVec,
    io_write_bytes: &GaugeVec,
) -> Result<()> {
    // io.stat format:
    // 8:0 rbytes=12345 wbytes=67890 rios=100 wios=200 dbytes=0 dios=0
    if let Ok(content) = fs::read_to_string(cgroup_path.join("io.stat")) {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let device = parts[0];

            for kv in &parts[1..] {
                if let Some((key, val)) = kv.split_once('=') {
                    if let Ok(v) = val.parse::<f64>() {
                        match key {
                            "rbytes" => {
                                io_read_bytes
                                    .with_label_values(&[cgroup_name, device])
                                    .set(v);
                            }
                            "wbytes" => {
                                io_write_bytes
                                    .with_label_values(&[cgroup_name, device])
                                    .set(v);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Parse cgroup v2 PIDs metrics
pub fn collect_pids_v2(
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
