# proctap — tiny Linux host metrics exporter (Rust + axum + Prometheus)

`proctap` scrapes high-signal, low-overhead metrics straight from `/proc` and `/sys`, and exposes them at `GET /metrics` for Prometheus scraping. Built in Rust with `axum` and the Prometheus client crate.

---

## Features

* **diskstat** — Block device I/O stats from /sys/class/block/<dev>/stat.
* **ethtool** — Driver/firmware NIC counters via `ethtool -S <iface>`.
* **interrupts** — IRQ distribution from /proc/interrupts (per IRQ × CPU).
* **memstat** — Memory info from /proc/meminfo.
* **netdev\_stat** — Per-interface NIC stats from /sys/class/net/<iface>/statistics/\*.
* **proc** — Process scheduler stats from /proc/<pid>/sched (per matching process).
* **queues** — Per-queue NIC sysfs values from /sys/class/net/<iface>/queues/{rx,tx}-\*/<files>.
* **snmp** — Kernel TCP/UDP/IP counters from /proc/net/snmp.
* **softirqs** — SoftIRQ totals from /proc/softirqs (per CPU).
* **softnet\_stat** — Per-CPU networking backlog stats from /proc/net/softnet\_stat.


---

## Quick start

### 1) Build & run (host)

```bash
# Requires Rust (stable)
cargo build --release

# Run the exporter
./target/release/proctap \
  --interval 5 \
  --proc-prefixes ping
```

Scrape at: `http://<host>:9000/metrics`.

> **Finding the right `--proc-prefixes`:** it matches Linux **`comm`** (truncated to 15 chars). Check with:
> `cat /proc/<PID>/comm`
> Example: if you run `ping 127.0.0.1`, the comm is likely `ping`.

---

## CLI

| Flag              | Default        | Description                                                                |
| ----------------- | -------------- | -------------------------------------------------------------------------- |
| `--interval`      | `5`            | Collection interval (seconds).                                             |
| `--proc-prefixes` | *(none)*       | Match `comm`; exporter publishes per-PID metrics for that process. |
| `--monitor`       | *(optional)*   | Comma-separated subset, e.g. `sched,net,disks,interrupts,meminfo`.         |

> Note: Linux truncates `comm` to 15 chars.

---

