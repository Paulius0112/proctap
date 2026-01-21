# proctap — comprehensive Linux host metrics exporter

`proctap` scrapes high-signal, low-overhead metrics straight from `/proc` and `/sys`, and exposes them at `GET /metrics` for Prometheus scraping. Built in Rust with `axum` and the Prometheus client crate.

**30 monitors** covering CPU, memory, network, I/O, and system metrics.

---

## Features

### Original Monitors
* **diskstat** — Block device I/O stats from `/sys/class/block/<dev>/stat`
* **ethtool** — Driver/firmware NIC counters via `ethtool -S <iface>`
* **interrupts** — IRQ distribution from `/proc/interrupts` (per IRQ × CPU)
* **memstat** — Memory info from `/proc/meminfo`
* **netdev_stat** — Per-interface NIC stats from `/sys/class/net/<iface>/statistics/*`
* **proc (sched)** — Process scheduler stats from `/proc/<pid>/sched`
* **queues** — Per-queue NIC sysfs values from `/sys/class/net/<iface>/queues/{rx,tx}-*`
* **snmp** — Kernel TCP/UDP/IP counters from `/proc/net/snmp`
* **softirqs** — SoftIRQ totals from `/proc/softirqs` (per CPU)
* **softnet_stat** — Per-CPU networking backlog stats from `/proc/net/softnet_stat`

### CPU Monitors
* **cpu-stat** — Per-CPU time (user/system/idle/iowait/irq/softirq/steal), context switches, processes
* **load-avg** — Load averages (1m, 5m, 15m), runnable/total tasks
* **cpu-freq** — CPU frequency (current, min, max, scaling)
* **cpu-idle** — CPU idle state usage and time per state
* **sched-stat** — Scheduler statistics per CPU

### Memory Monitors
* **vm-stat** — Virtual memory statistics (pgpgin, pgpgout, pswpin, pswpout, faults, THP)
* **zone-info** — NUMA zone pages and watermarks
* **buddy-info** — Memory fragmentation (buddy allocator free blocks per order)
* **slab-info** — Kernel slab cache statistics

### Process Monitor
* **proc-extended** — Unified process metrics: memory, I/O, CPU, scheduling, file descriptors, OOM score

### Network Monitors
* **net-stat** — Extended TCP/IP statistics (TcpExt, IpExt, MPTcpExt)
* **sock-stat** — Socket usage counts (TCP inuse/orphan/tw, UDP, etc.)
* **net-info** — Interface properties (speed, MTU, carrier, operstate)

### Block I/O Monitors
* **blk-queue** — Block device queue properties (nr_requests, read_ahead_kb, rotational)
* **blk-inflight** — In-flight I/O operations per device

### System Monitors
* **entropy** — Kernel entropy pool (available bits, pool size)
* **fs-stats** — File handles, inodes, dentries statistics
* **uptime** — System uptime and idle time
* **thermal** — Thermal zone temperatures

### cgroup Monitor
* **cgroup** — cgroup v1/v2 metrics: CPU, memory, I/O, PIDs (auto-detects version)

---

## Quick Start

### Build

```bash
# Requires Rust 1.88+
cargo build --release
```

### Run with Defaults (all monitors)

```bash
./target/release/proctap
```

Scrape at: `http://localhost:9000/metrics`

### Run with Specific Monitors

```bash
./target/release/proctap \
  --monitor mem-stat,cpu-stat,load-avg,vm-stat,disk-stat,uptime
```

### Run with Process Monitoring

```bash
# Monitor nginx and redis processes
./target/release/proctap \
  --proc-prefixes nginx,redis \
  --monitor sched,proc-extended,mem-stat,cpu-stat
```

### Run with Config File

```bash
./target/release/proctap --config /etc/proctap/proctap.toml
```

### Custom Port and Bind Address

```bash
./target/release/proctap --port 9100 --bind 127.0.0.1
```

---

## CLI Reference

| Flag | Default | Description |
|------|---------|-------------|
| `-c, --config` | — | Path to TOML config file |
| `-p, --port` | `9000` | Port to listen on |
| `-b, --bind` | `0.0.0.0` | Address to bind to |
| `-m, --monitor` | all | Comma-separated list of monitors to enable |
| `--interval` | `5` | Collection interval in seconds |
| `--proc-prefixes` | — | Process name prefixes for sched/proc-extended monitors |

### Available Monitors

```
sched, snmp, net-dev, net-dev-queues, disk-stat, interrupts, mem-stat,
soft-irqs, soft-net-stat, ethtool, cpu-stat, load-avg, cpu-freq, cpu-idle,
sched-stat, vm-stat, zone-info, buddy-info, slab-info, proc-extended,
net-stat, sock-stat, net-info, blk-queue, blk-inflight, entropy, fs-stats,
uptime, thermal, cgroup
```

---

## Configuration File

Create a TOML config file:

```toml
# /etc/proctap/proctap.toml

port = 9100
bind = "0.0.0.0"
interval = 5

# Optional: specify monitors (omit to enable all)
monitors = [
    "memstat",
    "cpustat",
    "loadavg",
    "vmstat",
    "diskstat",
    "netdev",
    "uptime",
    "thermal"
]

# Process prefixes for sched/proc_extended monitors
proc_prefixes = ["nginx", "redis", "postgres"]
```

CLI arguments override config file values.

---

## Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /metrics` | Prometheus metrics |
| `GET /health` | Health check (always returns 200 OK) |
| `GET /ready` | Readiness check (200 if metrics registered, 503 otherwise) |

---

## Docker Compose Test Environment

A complete test environment with Prometheus and Grafana is available in the `deploy/` directory.

> **Note:** This only works on Linux systems as proctap reads from `/proc` and `/sys`.

### Setup

```bash
# 1. Build proctap
cargo build --release

# 2. Start Prometheus and Grafana
cd deploy
docker compose up -d

# 3. Run proctap on the host (not in Docker)
../target/release/proctap --port 9100

# 4. Access services
# - Grafana: http://localhost:3000 (admin/admin)
# - Prometheus: http://localhost:9090
# - Proctap metrics: http://localhost:9100/metrics
```

The Grafana dashboard is automatically provisioned and available in the "Proctap" folder.

### Stop

```bash
docker compose down
```

---

## Grafana Dashboard

A comprehensive dashboard is included at `examples/grafana-dashboard.json` with panels for:

- System Overview (load, uptime, entropy, temperature)
- CPU (usage by mode, frequency, context switches, running/blocked processes)
- Memory (usage breakdown, page in/out, faults, swap activity)
- Disk I/O (throughput, in-flight operations)
- Network (throughput, socket stats, TCP retransmits/timeouts, softnet)
- File System (file handles, inodes, dentries)

### Manual Import

1. Open Grafana → Dashboards → Import
2. Upload `examples/grafana-dashboard.json`
3. Select your Prometheus data source

---

## Example Queries

### CPU Usage

```promql
# Total CPU usage percentage (excluding idle)
100 - (avg(rate(cpu_stat_seconds{mode="idle"}[1m])) * 100)

# CPU usage by mode
sum by (mode) (rate(cpu_stat_seconds{mode!="idle"}[1m])) * 100
```

### Memory

```promql
# Memory usage percentage
(1 - meminfo_bytes{key="MemAvailable"} / meminfo_bytes{key="MemTotal"}) * 100

# Swap usage
meminfo_bytes{key="SwapTotal"} - meminfo_bytes{key="SwapFree"}
```

### Disk I/O

```promql
# Disk read throughput
rate(disk_stat{key="read_bytes"}[1m])

# Disk write throughput
rate(disk_stat{key="write_bytes"}[1m])
```

### Network

```promql
# Network receive rate
rate(netdev_stat{key="rx_bytes", iface!="lo"}[1m])

# TCP retransmits
rate(netstat_tcp_ext{key="TCPRetransSegs"}[1m])
```

### Process Metrics (with --proc-prefixes)

```promql
# Process memory usage
proc_vm_rss_bytes{proc="nginx"}

# Process CPU time
rate(proc_utime_ticks{proc="nginx"}[1m]) + rate(proc_stime_ticks{proc="nginx"}[1m])

# Process file descriptors
proc_fd_count{proc="nginx"}
```

---

## Signal Handling

Proctap handles graceful shutdown on:
- `SIGTERM`
- `SIGINT` (Ctrl+C)

---

## Requirements

- **Linux** (reads from `/proc` and `/sys`)
- **Rust 1.88+** for building
- Optional: Docker & Docker Compose for test environment

---

## License

MIT
