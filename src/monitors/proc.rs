use std::{any, collections::BTreeMap, fs};

use prometheus::{Gauge, GaugeVec, Opts, Registry};

use crate::monitor::Monitor;


#[derive(Clone)]
pub struct ProcessSchedMonitor {
    proc_name_filter: String,
    nr_migrations: GaugeVec,
    nr_switches: GaugeVec,
    nr_involuntary_switches: GaugeVec,
    nr_voluntary_switches: GaugeVec,
}

impl ProcessSchedMonitor {
    pub fn new(registry: &Registry, proc_name: String) -> anyhow::Result<Self> {
        
        let make_gauge = |name, help| -> anyhow::Result<GaugeVec> {
            let g = GaugeVec::new(
                Opts::new(name, help), &["proc"]
            )?;

            registry.register(Box::new(g.clone()))?;
            Ok(g)
        };

        Ok(
            Self {
                proc_name_filter: proc_name,
                nr_migrations: make_gauge("proc_sched_nr_migrations", "se.nr_migrations from /proc/<pid>/sched")?,
                nr_switches: make_gauge("proc_sched_nr_switches", "nr_switches from /proc/<pid>/sched")?,
                nr_involuntary_switches: make_gauge(
                    "proc_sched_nr_involuntary_switches",
                    "nr_involuntary_switches from /proc/<pid>/sched",
                )?,
                nr_voluntary_switches: make_gauge(
                    "proc_sched_nr_voluntary_switches",
                    "nr_voluntary_switches from /proc/<pid>/sched",
                )?,
            }
        )
    }

    fn scan_pids(&self) -> anyhow::Result<Vec<String>> {
        let mut pids: Vec<String> = Vec::new();

        for entry in fs::read_dir("/proc")? {
            match entry {
                Ok(dir_entry) => {
                    if !dir_entry.file_type()?.is_dir() {continue;}
                    if dir_entry.file_name().to_string_lossy().parse::<u32>().is_err() { continue; }
                    let path = dir_entry.path();
                    let full_path = path.join("cmdline");

                    if fs::read_to_string(full_path)?.contains(&self.proc_name_filter) {
                        // Optimise this bit..
                        let pid: String = dir_entry.file_name().to_string_lossy().to_string();
                        pids.push(pid);
                    }
                },
                Err(e) => {
                    eprintln!("Error reading directory content: {}", e);
                }
            }
        }

        Ok(pids)
    }

    fn read_sched(&self, pid: &str) -> anyhow::Result<ProcessSched> {
        let content = fs::read_to_string(format!("/proc/{}/sched", pid))?;
        parse_process(content)
    }
}

impl Monitor for ProcessSchedMonitor {
    fn name(&self) -> &'static &str {
        &"sched"
    }

    fn collect(&mut self) -> anyhow::Result<()> {
        let proc_name = self.proc_name_filter.clone();

        let pids = self.scan_pids()?;

        for pid in pids {
            let label = &[proc_name.as_str()];
            let proc = self.read_sched(&pid)?;
            
            self.nr_involuntary_switches.with_label_values(label).set(proc.nr_involuntary_switches as f64);
            self.nr_migrations.with_label_values(label).set(proc.nr_migrations as f64);
            self.nr_voluntary_switches.with_label_values(label).set(proc.nr_voluntary_switches as f64);
            self.nr_migrations.with_label_values(label).set(proc.nr_migrations as f64);
        }

        Ok(())
    }
}

#[derive(Debug)]
struct ProcessSched {
    nr_migrations: u32,
    nr_switches: u32,
    nr_involuntary_switches: u32,
    nr_voluntary_switches: u32,
    prio: u16
}

impl ProcessSched {
    fn from_map(vals: &BTreeMap<&str, &str>) -> Option<ProcessSched> {
        Some(Self {
            nr_migrations: vals.get("se.nr_migrations")?.parse().ok()?,
            nr_switches: vals.get("nr_switches")?.parse().ok()?,
            nr_involuntary_switches: vals.get("nr_involuntary_switches")?.parse().ok()?,
            nr_voluntary_switches: vals.get("nr_voluntary_switches")?.parse().ok()?,
            prio: vals.get("prio")?.parse().ok()?,
        })
    }
}


fn parse_process(content: String) -> anyhow::Result<ProcessSched> {
    let mut lines = content.lines();
    
    // Handle without unwrap
    let header = lines.next().unwrap();

    let mut vals = BTreeMap::<&str, &str>::new();

    for line in lines {
        if !line.contains(":") {continue;}

        let row: Vec<&str> = line.trim().split(":").collect();
        let key = row[0].trim();
        let val = row[1].trim();
        
        vals.insert(key, val);
    }

    // This shouldn't ever be None, update
    Ok(ProcessSched::from_map(&vals).unwrap())
}