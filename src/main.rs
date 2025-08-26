use std::sync::Arc;
use std::time::Duration;
use std::vec;

use crate::monitor::{Monitor, MonitorKind};
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
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use clap::Parser;
use log::{error, info};
use prometheus::TextEncoder;
use prometheus::{HistogramOpts, HistogramVec, Opts, Registry};
use tokio::time::{interval, Instant};

mod monitor;
mod monitors;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(short = 'm', long = "monitor", value_delimiter = ',', value_enum)]
    monitors: Vec<MonitorKind>,
    #[arg(long, default_value_t = 5)]
    interval: u64,
    #[arg(long, default_value = "ping")]
    proc_name: String,
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let registry = Arc::new(Registry::new());

    let scrape_hist = HistogramVec::new(
        HistogramOpts {
            common_opts: Opts::new(
                "scrape_iteration_seconds",
                "Total time for one scrape iteration across all monitors",
            ),
            buckets: vec![
                0.00005, 0.0001, 0.0002, 0.0005, 0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5,
            ],
        },
        &["kind"],
    )?;
    registry.register(Box::new(scrape_hist.clone()))?;

    let enabled = if cli.monitors.is_empty() {
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
        ]
    } else {
        cli.monitors.clone()
    };

    let mut monitors: Vec<Box<dyn Monitor>> = Vec::new();
    for kind in enabled {
        match kind {
            MonitorKind::Sched => {
                monitors.push(Box::new(ProcessSchedMonitor::new(&registry, cli.proc_name.clone())?));
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
        }
    }

    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(AppState {
            registry: registry.clone(),
        });

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", 9000)).await?;
    let addr = listener.local_addr()?;
    info!("Serving Prometheus metrics on http://{addr}/metrics");

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!("metrics server error: {e:#}");
        }
    });

    let mut ticker = interval(Duration::from_secs(cli.interval));
    loop {
        ticker.tick().await;

        let iter_start = Instant::now();

        for mon in &mut monitors {
            if let Err(e) = mon.collect() {
                error!("Failed to collect metrics for: {e:#}");
            }
        }

        let iter_elapsed = iter_start.elapsed().as_secs_f64();
        scrape_hist.with_label_values(&["all"]).observe(iter_elapsed);
    }
}
