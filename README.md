# proctap — a tiny Linux host metrics exporter (Rust + axum + Prometheus)

`proctap` scrapes high-signal, low-overhead metrics straight from `/proc` and `/sys`, then exposes them at **`/metrics`** for Prometheus.

## Features

* **Process scheduler stats (per PID, per comm):** `/proc/<pid>/sched`
  Labels: `proc`, `pid`
* **SNMP stack counters (TCP/UDP only):** `/proc/net/snmp`
  Label: `key`
* **NIC counters (per interface):** `/sys/class/net/<iface>/statistics/*`
  Labels: `iface`, `key`
* **Disk I/O stats (per device):** `/sys/class/block/<dev>/stat`
  Labels: `dev`, `key`
* **Interrupt distribution (per IRQ × CPU):** `/proc/interrupts`
  Labels: `irq`, `cpu`, `name`
* **Meminfo (bytes & unitless keys):** `/proc/meminfo`
  Label: `key`

---

## Quick start

### Build & run

```bash
cargo build --release
./target/release/proctap \
  --listen 0.0.0.0:9000 \
  --interval 5 \
  --proc-name pinger
```

Scrape at: `http://<host>:9000/metrics`

### Example Prometheus scrape config

```yaml
scrape_configs:
  - job_name: 'proctap'
    static_configs:
      - targets: ['server1:9000']
```

---

## CLI

| Flag          | Default            | Description                                                                                                  |
| ------------- | ------------------ | ------------------------------------------------------------------------------------------------------------ |
| `--listen`    | `0.0.0.0:9000`     | HTTP bind for `/metrics`                                                                                     |
| `--interval`  | `5`                | Collection interval (seconds)                                                                                |
| `--proc-name` | `pinger` (example) | Match **/proc/\<pid>/comm** exactly; exporter publishes metrics for each matching PID (`proc`, `pid` labels) |
| `--monitor`   | *(optional)*       | Comma-separated subset (e.g., `sched,net,disks,interrupts,meminfo`) if you wired the enum toggles            |

> Note: Linux truncates `comm` to **15 chars**.

---

## What you’ll see (samples)

### Process scheduler (per process)

```
proc_sched_nr_switches{proc="pinger",pid="14764"} 372
proc_sched_nr_involuntary_switches{proc="pinger",pid="14764"} 8
proc_sched_nr_migrations{proc="pinger",pid="14764"} 24
proc_sum_exec_runtime{proc="pinger",pid="14764"} 53.155773
```

### TCP/UDP SNMP

```
snmp_tcp{key="RetransSegs"} 336
snmp_udp{key="InDatagrams"} 6070
snmp_udp{key="InErrors"} 0
```

### NIC stats (per iface)

```
netdev_stat{iface="enp34s0",key="rx_packets"} 0
netdev_stat{iface="wlo1",key="rx_bytes"} 434131102
```

### Disk stats (per device)

```
disk_stat{dev="nvme0n1",key="reads_completed"} 75894
disk_stat{dev="nvme0n1",key="io_time_ms"} 18320
disk_stat{dev="nvme0n1",key="weighted_io_time_ms"} 310647
```

### Interrupts (per IRQ × CPU)

```
interrupts{irq="24",cpu="3",name="enp34s0-TxRx-0"} 128934
```

### Meminfo

```
meminfo_bytes{key="MemTotal"} 3.361275904e+10
meminfo{key="HugePages_Total"} 0
```
