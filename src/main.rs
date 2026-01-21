use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::vec;

use crate::config::Config;
use crate::monitor::{Monitor, MonitorKind};
// Existing monitors
use crate::monitors::diskstat::DiskStatsMonitor;
use crate::monitors::ethtool::EthtoolMonitor;
use crate::monitors::interrupts::InterruptsMonitor;
use crate::monitors::memstat::MeminfoMonitor;
use crate::monitors::netdev_stat::NetSysfsStatsMonitor;
use crate::monitors::proc::ProcessSchedMonitor;
use crate::monitors::queues::NetSysfsQueuesMonitor;
use crate::monitors::snmp::SNMPMonitor;
use crate::monitors::softirqs::SoftirqsMonitor;
use crate::monitors::softnet_stat::SoftnetStatMonitor;
// CPU monitors
use crate::monitors::cpu_stat::CpuStatMonitor;
use crate::monitors::cpufreq::CpuFreqMonitor;
use crate::monitors::cpuidle::CpuIdleMonitor;
use crate::monitors::loadavg::LoadAvgMonitor;
use crate::monitors::schedstat::SchedStatMonitor;
// Memory monitors
use crate::monitors::buddyinfo::BuddyInfoMonitor;
use crate::monitors::slabinfo::SlabInfoMonitor;
use crate::monitors::vmstat::VmStatMonitor;
use crate::monitors::zoneinfo::ZoneInfoMonitor;
// Extended process monitor
use crate::monitors::proc_extended::ProcExtendedMonitor;
// Network monitors
use crate::monitors::netinfo::NetInfoMonitor;
use crate::monitors::netstat::NetStatMonitor;
use crate::monitors::sockstat::SockStatMonitor;
// Block I/O monitors
use crate::monitors::blkinflight::BlkInflightMonitor;
use crate::monitors::blkqueue::BlkQueueMonitor;
// System monitors
use crate::monitors::entropy::EntropyMonitor;
use crate::monitors::fs_stats::FsStatsMonitor;
use crate::monitors::thermal::ThermalMonitor;
use crate::monitors::uptime::UptimeMonitor;
// cgroup monitor
use crate::monitors::cgroup::CgroupMonitor;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use clap::Parser;
use log::{error, info, warn};
use prometheus::TextEncoder;
use prometheus::{HistogramOpts, HistogramVec, Opts, Registry};
use tokio::signal;
use tokio::sync::watch;
use tokio::time::{interval, Instant};

mod config;
mod monitor;
mod monitors;
mod parsers;

#[derive(Parser, Debug)]
#[command(name = "proctap")]
#[command(about = "Linux low-level metrics monitoring for Prometheus")]
struct Cli {
    /// Config file path
    #[arg(short = 'c', long = "config")]
    config: Option<PathBuf>,

    /// Port to listen on (overrides config file)
    #[arg(short = 'p', long = "port")]
    port: Option<u16>,

    /// Address to bind to (overrides config file)
    #[arg(short = 'b', long = "bind")]
    bind: Option<String>,

    /// Monitors to enable (comma-separated)
    #[arg(short = 'm', long = "monitor", value_delimiter = ',', value_enum)]
    monitors: Vec<MonitorKind>,

    /// Collection interval in seconds
    #[arg(long, default_value_t = 5)]
    interval: u64,

    /// Process name prefixes for sched/proc_extended monitors
    #[arg(long, value_delimiter = ',')]
    proc_prefixes: Vec<String>,
}

#[derive(Clone)]
struct AppState {
    registry: Arc<Registry>,
}

async fn metrics_handler(State(state): State<AppState>) -> (StatusCode, String) {
    let metric_families = state.registry.gather();
    let mut body = String::new();
    TextEncoder::new()
        .encode_utf8(&metric_families, &mut body)
        .expect("encode metrics");
    (StatusCode::OK, body)
}

async fn health_handler() -> StatusCode {
    StatusCode::OK
}

async fn ready_handler(State(state): State<AppState>) -> StatusCode {
    // Check if registry has any metrics registered
    let families = state.registry.gather();
    if families.is_empty() {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    // Load config file if specified
    let file_config = if let Some(config_path) = &cli.config {
        match Config::load(config_path) {
            Ok(c) => {
                info!("Loaded config from {}", config_path.display());
                c
            }
            Err(e) => {
                warn!("Failed to load config file: {}, using defaults", e);
                Config::default()
            }
        }
    } else {
        Config::default()
    };

    // CLI args override config file
    let port = cli.port.unwrap_or(file_config.port);
    let bind = cli.bind.clone().unwrap_or_else(|| file_config.bind.clone());
    let collection_interval = if cli.interval != 5 {
        cli.interval
    } else {
        file_config.interval
    };
    let proc_prefixes = if !cli.proc_prefixes.is_empty() {
        cli.proc_prefixes.clone()
    } else {
        file_config.proc_prefixes.clone()
    };

    let registry = Arc::new(Registry::new());

    let scrape_hist = HistogramVec::new(
        HistogramOpts {
            common_opts: Opts::new(
                "scrape_iteration_seconds",
                "Total time for one scrape iteration across all monitors",
            ),
            buckets: vec![
                0.00005, 0.0001, 0.0002, 0.0005, 0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1, 0.25,
                0.5, 1.0, 2.5,
            ],
        },
        &["kind"],
    )?;
    registry.register(Box::new(scrape_hist.clone()))?;

    // Determine which monitors to enable
    let enabled = if !cli.monitors.is_empty() {
        cli.monitors.clone()
    } else if !file_config.monitors.is_empty() {
        file_config.parse_monitors()
    } else {
        // Default: enable all monitors
        vec![
            MonitorKind::Sched,
            MonitorKind::Snmp,
            MonitorKind::NetDev,
            MonitorKind::DiskStat,
            MonitorKind::Interrupts,
            MonitorKind::MemStat,
            MonitorKind::NetDevQueues,
            MonitorKind::SoftIrqs,
            MonitorKind::SoftNetStat,
            MonitorKind::Ethtool,
            MonitorKind::CpuStat,
            MonitorKind::LoadAvg,
            MonitorKind::CpuFreq,
            MonitorKind::CpuIdle,
            MonitorKind::SchedStat,
            MonitorKind::VmStat,
            MonitorKind::ZoneInfo,
            MonitorKind::BuddyInfo,
            MonitorKind::SlabInfo,
            MonitorKind::ProcExtended,
            MonitorKind::NetStat,
            MonitorKind::SockStat,
            MonitorKind::NetInfo,
            MonitorKind::BlkQueue,
            MonitorKind::BlkInflight,
            MonitorKind::Entropy,
            MonitorKind::FsStats,
            MonitorKind::Uptime,
            MonitorKind::Thermal,
            MonitorKind::Cgroup,
        ]
    };

    let mut monitors: Vec<Box<dyn Monitor>> = Vec::new();
    for kind in enabled {
        match kind {
            // Existing monitors
            MonitorKind::Sched => {
                if !proc_prefixes.is_empty() {
                    monitors.push(Box::new(ProcessSchedMonitor::new(
                        &registry,
                        proc_prefixes.clone(),
                    )?));
                }
            }
            MonitorKind::Snmp => {
                monitors.push(Box::new(SNMPMonitor::new(&registry)?));
            }
            MonitorKind::NetDev => {
                monitors.push(Box::new(NetSysfsStatsMonitor::new(&registry)?));
            }
            MonitorKind::DiskStat => {
                monitors.push(Box::new(DiskStatsMonitor::new(&registry)?));
            }
            MonitorKind::Interrupts => {
                monitors.push(Box::new(InterruptsMonitor::new(&registry)?));
            }
            MonitorKind::MemStat => {
                monitors.push(Box::new(MeminfoMonitor::new(&registry)?));
            }
            MonitorKind::NetDevQueues => {
                monitors.push(Box::new(NetSysfsQueuesMonitor::new(&registry)?));
            }
            MonitorKind::SoftIrqs => {
                monitors.push(Box::new(SoftirqsMonitor::new(&registry)?));
            }
            MonitorKind::SoftNetStat => {
                monitors.push(Box::new(SoftnetStatMonitor::new(&registry)?));
            }
            MonitorKind::Ethtool => {
                monitors.push(Box::new(EthtoolMonitor::new(&registry)?));
            }
            // CPU monitors
            MonitorKind::CpuStat => {
                monitors.push(Box::new(CpuStatMonitor::new(&registry)?));
            }
            MonitorKind::LoadAvg => {
                monitors.push(Box::new(LoadAvgMonitor::new(&registry)?));
            }
            MonitorKind::CpuFreq => {
                monitors.push(Box::new(CpuFreqMonitor::new(&registry)?));
            }
            MonitorKind::CpuIdle => {
                monitors.push(Box::new(CpuIdleMonitor::new(&registry)?));
            }
            MonitorKind::SchedStat => {
                monitors.push(Box::new(SchedStatMonitor::new(&registry)?));
            }
            // Memory monitors
            MonitorKind::VmStat => {
                monitors.push(Box::new(VmStatMonitor::new(&registry)?));
            }
            MonitorKind::ZoneInfo => {
                monitors.push(Box::new(ZoneInfoMonitor::new(&registry)?));
            }
            MonitorKind::BuddyInfo => {
                monitors.push(Box::new(BuddyInfoMonitor::new(&registry)?));
            }
            MonitorKind::SlabInfo => {
                monitors.push(Box::new(SlabInfoMonitor::new(&registry)?));
            }
            // Extended process monitor
            MonitorKind::ProcExtended => {
                if !proc_prefixes.is_empty() {
                    monitors.push(Box::new(ProcExtendedMonitor::new(
                        &registry,
                        proc_prefixes.clone(),
                    )?));
                }
            }
            // Network monitors
            MonitorKind::NetStat => {
                monitors.push(Box::new(NetStatMonitor::new(&registry)?));
            }
            MonitorKind::SockStat => {
                monitors.push(Box::new(SockStatMonitor::new(&registry)?));
            }
            MonitorKind::NetInfo => {
                monitors.push(Box::new(NetInfoMonitor::new(&registry)?));
            }
            // Block I/O monitors
            MonitorKind::BlkQueue => {
                monitors.push(Box::new(BlkQueueMonitor::new(&registry)?));
            }
            MonitorKind::BlkInflight => {
                monitors.push(Box::new(BlkInflightMonitor::new(&registry)?));
            }
            // System monitors
            MonitorKind::Entropy => {
                monitors.push(Box::new(EntropyMonitor::new(&registry)?));
            }
            MonitorKind::FsStats => {
                monitors.push(Box::new(FsStatsMonitor::new(&registry)?));
            }
            MonitorKind::Uptime => {
                monitors.push(Box::new(UptimeMonitor::new(&registry)?));
            }
            MonitorKind::Thermal => {
                monitors.push(Box::new(ThermalMonitor::new(&registry)?));
            }
            // cgroup monitor
            MonitorKind::Cgroup => {
                monitors.push(Box::new(CgroupMonitor::new(&registry)?));
            }
        }
    }

    info!("Initialized {} monitors", monitors.len());

    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .with_state(AppState {
            registry: registry.clone(),
        });

    let listener = tokio::net::TcpListener::bind((bind.as_str(), port)).await?;
    let addr = listener.local_addr()?;
    info!("Serving Prometheus metrics on http://{addr}/metrics");
    info!("Health endpoint: http://{addr}/health");
    info!("Ready endpoint: http://{addr}/ready");

    // Create shutdown channel
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    // Spawn server with graceful shutdown
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown_rx.changed().await.ok();
            })
            .await
    });

    // Collection loop with shutdown handling
    let mut ticker = interval(Duration::from_secs(collection_interval));
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let iter_start = Instant::now();

                for mon in &mut monitors {
                    if let Err(e) = mon.collect() {
                        error!("Failed to collect metrics for: {e:#}");
                    }
                }

                let iter_elapsed = iter_start.elapsed().as_secs_f64();
                scrape_hist.with_label_values(&["all"]).observe(iter_elapsed);
            }
            _ = shutdown_signal() => {
                info!("Shutdown signal received, stopping...");
                break;
            }
        }
    }

    // Signal server to shutdown
    let _ = shutdown_tx.send(true);

    // Wait for server to finish
    match server.await {
        Ok(Ok(())) => info!("Server shut down gracefully"),
        Ok(Err(e)) => error!("Server error during shutdown: {e}"),
        Err(e) => error!("Server task panicked: {e}"),
    }

    info!("Proctap stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
