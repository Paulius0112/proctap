#![allow(dead_code)]

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Parse a file with "Key: Value" format (like /proc/meminfo, /proc/<pid>/status)
/// Returns a HashMap of key -> value strings
pub fn parse_kv_file(path: &Path) -> Result<HashMap<String, String>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    parse_kv_content(&content)
}

/// Parse content with "Key: Value" format
pub fn parse_kv_content(content: &str) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for line in content.lines() {
        if let Some((k, v)) = line.split_once(':') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    Ok(map)
}

/// Parse /proc/<pid>/stat which has a tricky format with comm field in parentheses
/// Returns (pid, comm, state, and remaining fields as Vec<&str>)
pub fn parse_proc_pid_stat(content: &str) -> Option<(u32, String, char, Vec<&str>)> {
    // Format: pid (comm) state ppid pgrp session tty_nr tpgid flags ...
    // comm can contain spaces and parentheses
    let open_paren = content.find('(')?;
    let close_paren = content.rfind(')')?;

    let pid_str = content[..open_paren].trim();
    let pid: u32 = pid_str.parse().ok()?;

    let comm = content[open_paren + 1..close_paren].to_string();

    let rest = content[close_paren + 1..].trim();
    let mut fields: Vec<&str> = rest.split_whitespace().collect();

    if fields.is_empty() {
        return None;
    }

    let state = fields.remove(0).chars().next()?;

    Some((pid, comm, state, fields))
}

/// Parse space-separated numeric values from a line
pub fn parse_space_separated_u64(line: &str) -> Vec<u64> {
    line.split_whitespace()
        .filter_map(|s| s.parse::<u64>().ok())
        .collect()
}

/// Parse space-separated f64 values from a line
pub fn parse_space_separated_f64(line: &str) -> Vec<f64> {
    line.split_whitespace()
        .filter_map(|s| s.parse::<f64>().ok())
        .collect()
}

/// Read a single numeric value from a sysfs file
pub fn read_sysfs_u64(path: &Path) -> Result<u64> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    content
        .trim()
        .parse::<u64>()
        .with_context(|| format!("Failed to parse {} as u64", path.display()))
}

/// Read a single numeric value from a sysfs file, returning None on error
pub fn read_sysfs_u64_opt(path: &Path) -> Option<u64> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Read a single f64 value from a sysfs file
pub fn read_sysfs_f64(path: &Path) -> Result<f64> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    content
        .trim()
        .parse::<f64>()
        .with_context(|| format!("Failed to parse {} as f64", path.display()))
}

/// Read a string value from a sysfs file
pub fn read_sysfs_string(path: &Path) -> Result<String> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    Ok(content.trim().to_string())
}

/// Read a string value from a sysfs file, returning None on error
pub fn read_sysfs_string_opt(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// Parse a size value with optional unit suffix (K, M, G)
pub fn parse_size_with_unit(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (num_str, multiplier) = if s.ends_with('K') || s.ends_with('k') {
        (&s[..s.len() - 1], 1024u64)
    } else if s.ends_with('M') || s.ends_with('m') {
        (&s[..s.len() - 1], 1024 * 1024)
    } else if s.ends_with('G') || s.ends_with('g') {
        (&s[..s.len() - 1], 1024 * 1024 * 1024)
    } else {
        (s, 1)
    };

    num_str.trim().parse::<u64>().ok().map(|n| n * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kv_content() {
        let content = "MemTotal:       16336236 kB\nMemFree:         1234567 kB";
        let map = parse_kv_content(content).unwrap();
        assert_eq!(map.get("MemTotal"), Some(&"16336236 kB".to_string()));
        assert_eq!(map.get("MemFree"), Some(&"1234567 kB".to_string()));
    }

    #[test]
    fn test_parse_proc_pid_stat() {
        let content = "1234 (my process) S 1 1234 1234 0 -1 4194304";
        let result = parse_proc_pid_stat(content).unwrap();
        assert_eq!(result.0, 1234);
        assert_eq!(result.1, "my process");
        assert_eq!(result.2, 'S');
        // Fields after state: "1 1234 1234 0 -1 4194304" = 6 fields
        assert_eq!(result.3.len(), 6);
    }

    #[test]
    fn test_parse_proc_pid_stat_with_parens_in_comm() {
        let content = "5678 (bash (test)) R 5677 5678 5678 34818 5678 4194304";
        let result = parse_proc_pid_stat(content).unwrap();
        assert_eq!(result.0, 5678);
        assert_eq!(result.1, "bash (test)");
        assert_eq!(result.2, 'R');
    }

    #[test]
    fn test_parse_space_separated_u64() {
        let line = "123 456 789 0";
        let vals = parse_space_separated_u64(line);
        assert_eq!(vals, vec![123, 456, 789, 0]);
    }

    #[test]
    fn test_parse_size_with_unit() {
        assert_eq!(parse_size_with_unit("1024"), Some(1024));
        assert_eq!(parse_size_with_unit("1K"), Some(1024));
        assert_eq!(parse_size_with_unit("1M"), Some(1024 * 1024));
        assert_eq!(parse_size_with_unit("2G"), Some(2 * 1024 * 1024 * 1024));
    }
}
