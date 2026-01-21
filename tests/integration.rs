#[allow(dead_code)]
mod fixtures;

/// Test that we can parse /proc/loadavg format correctly
#[test]
fn test_loadavg_parsing() {
    // Format: "load_1m load_5m load_15m runnable/total last_pid"
    let content = "0.01 0.05 0.10 1/234 5678";
    let parts: Vec<&str> = content.split_whitespace().collect();

    assert_eq!(parts.len(), 5);
    assert_eq!(parts[0].parse::<f64>().unwrap(), 0.01);
    assert_eq!(parts[1].parse::<f64>().unwrap(), 0.05);
    assert_eq!(parts[2].parse::<f64>().unwrap(), 0.10);

    let (runnable, total) = parts[3].split_once('/').unwrap();
    assert_eq!(runnable.parse::<u32>().unwrap(), 1);
    assert_eq!(total.parse::<u32>().unwrap(), 234);
    assert_eq!(parts[4].parse::<u32>().unwrap(), 5678);
}

/// Test that we can parse /proc/uptime format correctly
#[test]
fn test_uptime_parsing() {
    // Format: "uptime_seconds idle_seconds"
    let content = "12345.67 98765.43";
    let parts: Vec<&str> = content.split_whitespace().collect();

    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].parse::<f64>().unwrap(), 12345.67);
    assert_eq!(parts[1].parse::<f64>().unwrap(), 98765.43);
}

/// Test that we can parse /proc/stat CPU lines correctly
#[test]
fn test_cpu_stat_parsing() {
    // Format: "cpuN user nice system idle iowait irq softirq steal guest guest_nice"
    let line = "cpu0 1234 567 890 12345 678 90 12 3 4 5";
    let parts: Vec<&str> = line.split_whitespace().collect();

    assert_eq!(parts[0], "cpu0");
    assert!(parts.len() >= 8); // At minimum 7 values after cpu name

    let user: u64 = parts[1].parse().unwrap();
    let nice: u64 = parts[2].parse().unwrap();
    let system: u64 = parts[3].parse().unwrap();
    let idle: u64 = parts[4].parse().unwrap();

    assert_eq!(user, 1234);
    assert_eq!(nice, 567);
    assert_eq!(system, 890);
    assert_eq!(idle, 12345);
}

/// Test that we can parse /proc/vmstat format correctly
#[test]
fn test_vmstat_parsing() {
    let content = "pgpgin 12345\npgpgout 67890\npswpin 0\npswpout 100";
    let mut count = 0;

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(parts.len(), 2);
        let _key = parts[0];
        let _value: u64 = parts[1].parse().unwrap();
        count += 1;
    }

    assert_eq!(count, 4);
}

/// Test that we can parse /proc/buddyinfo format correctly
#[test]
fn test_buddyinfo_parsing() {
    let line = "Node 0, zone      DMA      1      1      1      0      2      1      1      0      1      1      3";

    assert!(line.starts_with("Node "));
    let parts: Vec<&str> = line.splitn(2, ", zone").collect();
    assert_eq!(parts.len(), 2);

    let node = parts[0].strip_prefix("Node ").unwrap().trim();
    assert_eq!(node, "0");

    let zone_and_values: Vec<&str> = parts[1].split_whitespace().collect();
    assert_eq!(zone_and_values[0], "DMA");

    // Remaining values are free block counts at each order
    for count_str in zone_and_values.iter().skip(1) {
        let _count: u64 = count_str.parse().unwrap();
    }
}

/// Test that we can parse /proc/net/sockstat format correctly
#[test]
fn test_sockstat_parsing() {
    let content = "sockets: used 123\nTCP: inuse 45 orphan 0 tw 67 alloc 89 mem 123\nUDP: inuse 10 mem 5";

    for line in content.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        assert_eq!(parts.len(), 2);

        let protocol = parts[0].trim();
        assert!(!protocol.is_empty());

        let kv_parts: Vec<&str> = parts[1].split_whitespace().collect();

        // Parse key-value pairs
        let mut i = 0;
        while i + 1 < kv_parts.len() {
            let _key = kv_parts[i];
            let _val: u64 = kv_parts[i + 1].parse().unwrap();
            i += 2;
        }
    }
}

/// Test that we can parse /proc/<pid>/stat format correctly (tricky due to comm field)
#[test]
fn test_proc_pid_stat_parsing() {
    // The comm field can contain spaces and parentheses
    let content = "1234 (my process) S 1 1234 1234 0 -1 4194304 100 0 0 0 10 5 0 0 20 0 1 0 1000";

    let open_paren = content.find('(').unwrap();
    let close_paren = content.rfind(')').unwrap();

    let pid_str = content[..open_paren].trim();
    assert_eq!(pid_str.parse::<u32>().unwrap(), 1234);

    let comm = &content[open_paren + 1..close_paren];
    assert_eq!(comm, "my process");

    let rest = content[close_paren + 1..].trim();
    let fields: Vec<&str> = rest.split_whitespace().collect();
    assert_eq!(fields[0], "S"); // state
    assert!(!fields.is_empty());
}

/// Test that we can parse /proc/<pid>/stat with parens in comm field
#[test]
fn test_proc_pid_stat_with_parens() {
    let content = "5678 (bash (test)) R 5677 5678 5678 34818 5678 4194304 100 0 0 0 10 5 0 0 20 0 1 0 1000";

    let open_paren = content.find('(').unwrap();
    let close_paren = content.rfind(')').unwrap();

    let pid_str = content[..open_paren].trim();
    assert_eq!(pid_str.parse::<u32>().unwrap(), 5678);

    let comm = &content[open_paren + 1..close_paren];
    assert_eq!(comm, "bash (test)");
}

/// Test that we can parse /proc/schedstat format correctly
#[test]
fn test_schedstat_parsing() {
    // Format varies by kernel version, but cpu lines are consistent
    let line = "cpu0 0 0 1234567 0 0 0 890123456 7890123456";

    assert!(line.starts_with("cpu"));
    let parts: Vec<&str> = line.split_whitespace().collect();
    assert!(parts.len() >= 8);

    let cpu_name = parts[0];
    assert_eq!(cpu_name, "cpu0");

    // Parse numeric fields
    for field in &parts[1..] {
        let _val: u64 = field.parse().unwrap();
    }
}

/// Test parsing of /proc/net/netstat paired header/value format
#[test]
fn test_netstat_parsing() {
    let content = "TcpExt: SyncookiesSent SyncookiesRecv SyncookiesFailed\nTcpExt: 0 0 100\nIpExt: InNoRoutes InTruncatedPkts\nIpExt: 0 0";

    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 4);

    let mut i = 0;
    while i + 1 < lines.len() {
        let header = lines[i];
        let values = lines[i + 1];

        let header_parts: Vec<&str> = header.split_whitespace().collect();
        let value_parts: Vec<&str> = values.split_whitespace().collect();

        // Both should start with same protocol
        let proto = header_parts[0].trim_end_matches(':');
        assert!(values.starts_with(proto));

        // Should have same number of fields
        assert_eq!(header_parts.len(), value_parts.len());

        i += 2;
    }
}

/// Test config file parsing
#[test]
fn test_config_parsing() {
    let toml_content = r#"
port = 8080
bind = "127.0.0.1"
interval = 10
monitors = ["memstat", "cpustat", "loadavg"]
proc_prefixes = ["nginx", "redis"]
"#;

    // Use toml crate to parse
    let config: toml::Value = toml::from_str(toml_content).unwrap();

    assert_eq!(config["port"].as_integer().unwrap(), 8080);
    assert_eq!(config["bind"].as_str().unwrap(), "127.0.0.1");
    assert_eq!(config["interval"].as_integer().unwrap(), 10);

    let monitors = config["monitors"].as_array().unwrap();
    assert_eq!(monitors.len(), 3);
    assert_eq!(monitors[0].as_str().unwrap(), "memstat");

    let prefixes = config["proc_prefixes"].as_array().unwrap();
    assert_eq!(prefixes.len(), 2);
    assert_eq!(prefixes[0].as_str().unwrap(), "nginx");
}

/// Test that we handle empty config gracefully
#[test]
fn test_empty_config() {
    let toml_content = "";
    let config: toml::Value = toml::from_str(toml_content).unwrap();
    assert!(config.as_table().unwrap().is_empty());
}

/// Test cgroup v2 cpu.stat parsing
#[test]
fn test_cgroup_v2_cpu_stat() {
    let content = "usage_usec 12345678\nuser_usec 6789012\nsystem_usec 5456789\nnr_periods 0\nnr_throttled 0\nthrottled_usec 0";

    for line in content.lines() {
        if let Some((key, val)) = line.split_once(' ') {
            match key {
                "usage_usec" | "user_usec" | "system_usec" => {
                    let _v: u64 = val.parse().unwrap();
                }
                _ => {}
            }
        }
    }
}

/// Test cgroup v2 io.stat parsing
#[test]
fn test_cgroup_v2_io_stat() {
    let content = "8:0 rbytes=12345 wbytes=67890 rios=100 wios=200 dbytes=0 dios=0";

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let device = parts[0];
        assert_eq!(device, "8:0");

        for kv in &parts[1..] {
            if let Some((key, val)) = kv.split_once('=') {
                let _v: u64 = val.parse().unwrap();
                assert!(!key.is_empty());
            }
        }
    }
}

/// Test thermal zone temp parsing (millicelsius to celsius)
#[test]
fn test_thermal_temp_parsing() {
    let millicelsius = "45000";
    let celsius = millicelsius.parse::<i64>().unwrap() as f64 / 1000.0;
    assert_eq!(celsius, 45.0);

    let millicelsius = "-5000"; // Some zones can report negative
    let celsius = millicelsius.parse::<i64>().unwrap() as f64 / 1000.0;
    assert_eq!(celsius, -5.0);
}

/// Test /proc/sys/fs/file-nr parsing
#[test]
fn test_file_nr_parsing() {
    let content = "1234\t0\t65536";
    let parts: Vec<&str> = content.split_whitespace().collect();

    assert_eq!(parts.len(), 3);
    let allocated: u64 = parts[0].parse().unwrap();
    let free: u64 = parts[1].parse().unwrap();
    let max: u64 = parts[2].parse().unwrap();

    assert_eq!(allocated, 1234);
    assert_eq!(free, 0);
    assert_eq!(max, 65536);
}

/// Test /sys/block/*/inflight parsing
#[test]
fn test_blk_inflight_parsing() {
    let content = "0 5";
    let parts: Vec<&str> = content.split_whitespace().collect();

    assert_eq!(parts.len(), 2);
    let read_inflight: u64 = parts[0].parse().unwrap();
    let write_inflight: u64 = parts[1].parse().unwrap();

    assert_eq!(read_inflight, 0);
    assert_eq!(write_inflight, 5);
}
