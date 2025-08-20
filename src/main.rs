use std::sync::Arc;
use std::time::Duration;
use std::vec;

use crate::monitor::{Monitor, MonitorKind};
use crate::monitors::diskstat::DiskStatsMonitor;
use crate::monitors::interrupts::InterruptsMonitor;
use crate::monitors::memstat::MeminfoMonitor;
use crate::monitors::netdev_stat::NetSysfsStatsMonitor;
use crate::monitors::proc::ProcessSchedMonitor;
use crate::monitors::snmp::SNMPMonitor;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use clap::Parser;
use log::{error, info};
use prometheus::Registry;
use prometheus::TextEncoder;
use tokio::time::interval;

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

    let enabled = if cli.monitors.is_empty() {
        vec![
            MonitorKind::Sched,
            MonitorKind::Snmp,
            MonitorKind::NetDev,
            MonitorKind::DiskStat,
            MonitorKind::Interrupts,
            MonitorKind::MemStat,
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
        }
    }

    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(AppState {
            registry: registry.clone(),
        });

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", 9000)).await?;
    info!("Serving Prometheus metrics on {listener:?}");
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!("metrics server error: {e:#}");
        }
    });

    let mut ticker = interval(Duration::from_secs(cli.interval));
    loop {
        ticker.tick().await;

        for mon in &mut monitors {
            if let Err(e) = mon.collect() {
                error!("Failed to collect metrics for: {e:#}");
            }
        }
    }
}
