
use std::sync::Arc;
use std::time::Duration;
use std::vec;
use std::{any, collections::BTreeMap, fmt::format, fs, process};

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use log::info;
use prometheus::Registry;
use prometheus::TextEncoder;
use serde::Deserialize;
use clap::Parser;
use clap::ValueEnum;
use tokio::net::unix::SocketAddr;
use tokio::time::interval;
use crate::monitor::{Monitor, MonitorKind};
use crate::monitors::net::SNMPMonitor;
use crate::monitors::proc::ProcessSchedMonitor;


mod monitor;
mod monitors;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(short = 'm', long = "monitor", value_delimiter = ',', value_enum)]
    monitors: Vec<MonitorKind>,
    #[arg(long, default_value_t = 5)]
    interval: u64,
    #[arg(long, default_value = "target/debug/pinger")]
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
        vec![MonitorKind::Sched, MonitorKind::Net]
    } else {
        cli.monitors.clone()
    };

    let mut monitors: Vec<Box<dyn Monitor>> = Vec::new();
    for kind in enabled {
        match kind {
            MonitorKind::Sched => {
                monitors.push(Box::new(ProcessSchedMonitor::new(&registry, cli.proc_name.clone())?));
            },
            MonitorKind::Net => {
                monitors.push(Box::new(SNMPMonitor::new(&registry)?));
            }
            _ => {}
        }
    }

    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(AppState { registry: registry.clone() });

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", 9000)).await?;
    info!("Serving Prometheus metrics on {:?}", listener);
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("metrics server error: {e:#}");
        }
    });


    let mut ticker = interval(Duration::from_secs(cli.interval));
    loop {
        ticker.tick().await;

        for mon in &mut monitors {
            if let Err(e) = mon.collect() {
                eprintln!("Failed to collect metrics for: {e:#}");
            }
        }
    }
}