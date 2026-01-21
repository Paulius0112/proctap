use anyhow::Result;
use log::{debug, info};
use prometheus::{GaugeVec, Opts, Registry};
use std::fs;

use crate::monitor::Monitor;

pub struct SlabInfoMonitor {
    active_objs: GaugeVec, // labels: [slab]
    num_objs: GaugeVec,    // labels: [slab]
    objsize: GaugeVec,     // labels: [slab]
    objperslab: GaugeVec,  // labels: [slab]
    pagesperslab: GaugeVec, // labels: [slab]
    active_slabs: GaugeVec, // labels: [slab]
    num_slabs: GaugeVec,   // labels: [slab]
}

impl SlabInfoMonitor {
    pub fn new(registry: &Registry) -> Result<Self> {
        info!("Enabled SlabInfo monitor");

        let active_objs = GaugeVec::new(
            Opts::new(
                "slabinfo_active_objects",
                "Number of active objects in the slab cache",
            ),
            &["slab"],
        )?;
        let num_objs = GaugeVec::new(
            Opts::new(
                "slabinfo_total_objects",
                "Total number of objects in the slab cache",
            ),
            &["slab"],
        )?;
        let objsize = GaugeVec::new(
            Opts::new(
                "slabinfo_object_size_bytes",
                "Size of each object in the slab cache",
            ),
            &["slab"],
        )?;
        let objperslab = GaugeVec::new(
            Opts::new(
                "slabinfo_objects_per_slab",
                "Number of objects per slab",
            ),
            &["slab"],
        )?;
        let pagesperslab = GaugeVec::new(
            Opts::new(
                "slabinfo_pages_per_slab",
                "Number of pages per slab",
            ),
            &["slab"],
        )?;
        let active_slabs = GaugeVec::new(
            Opts::new(
                "slabinfo_active_slabs",
                "Number of active slabs",
            ),
            &["slab"],
        )?;
        let num_slabs = GaugeVec::new(
            Opts::new(
                "slabinfo_total_slabs",
                "Total number of slabs",
            ),
            &["slab"],
        )?;

        registry.register(Box::new(active_objs.clone()))?;
        registry.register(Box::new(num_objs.clone()))?;
        registry.register(Box::new(objsize.clone()))?;
        registry.register(Box::new(objperslab.clone()))?;
        registry.register(Box::new(pagesperslab.clone()))?;
        registry.register(Box::new(active_slabs.clone()))?;
        registry.register(Box::new(num_slabs.clone()))?;

        Ok(Self {
            active_objs,
            num_objs,
            objsize,
            objperslab,
            pagesperslab,
            active_slabs,
            num_slabs,
        })
    }

    fn collect_once(&self) -> Result<()> {
        // Format (version 2.1):
        // slabinfo - version: 2.1
        // # name            <active_objs> <num_objs> <objsize> <objperslab> <pagesperslab> : tunables ...
        let content = fs::read_to_string("/proc/slabinfo")?;
        let mut slab_count = 0usize;

        for line in content.lines() {
            // Skip header lines
            if line.starts_with("slabinfo") || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 6 {
                continue;
            }

            let slab_name = parts[0];

            // Parse numeric fields
            let active_objs: Option<u64> = parts.get(1).and_then(|s| s.parse().ok());
            let num_objs: Option<u64> = parts.get(2).and_then(|s| s.parse().ok());
            let objsize: Option<u64> = parts.get(3).and_then(|s| s.parse().ok());
            let objperslab: Option<u64> = parts.get(4).and_then(|s| s.parse().ok());
            let pagesperslab: Option<u64> = parts.get(5).and_then(|s| s.parse().ok());

            // Find active_slabs and num_slabs after the colon
            let colon_pos = parts.iter().position(|&s| s == ":");
            let (active_slabs, num_slabs): (Option<u64>, Option<u64>) = if let Some(pos) = colon_pos {
                // After ":" comes tunables, then ":" again, then slabdata
                // Format: : tunables <...> : slabdata <active_slabs> <num_slabs> ...
                let after_colon = &parts[pos + 1..];
                if let Some(slabdata_pos) = after_colon.iter().position(|&s| s == "slabdata") {
                    let slabdata = &after_colon[slabdata_pos + 1..];
                    (
                        slabdata.first().and_then(|s| s.parse().ok()),
                        slabdata.get(1).and_then(|s| s.parse().ok()),
                    )
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            if let Some(v) = active_objs {
                self.active_objs
                    .with_label_values(&[slab_name])
                    .set(v as f64);
            }
            if let Some(v) = num_objs {
                self.num_objs.with_label_values(&[slab_name]).set(v as f64);
            }
            if let Some(v) = objsize {
                self.objsize.with_label_values(&[slab_name]).set(v as f64);
            }
            if let Some(v) = objperslab {
                self.objperslab
                    .with_label_values(&[slab_name])
                    .set(v as f64);
            }
            if let Some(v) = pagesperslab {
                self.pagesperslab
                    .with_label_values(&[slab_name])
                    .set(v as f64);
            }
            if let Some(v) = active_slabs {
                self.active_slabs
                    .with_label_values(&[slab_name])
                    .set(v as f64);
            }
            if let Some(v) = num_slabs {
                self.num_slabs
                    .with_label_values(&[slab_name])
                    .set(v as f64);
            }

            slab_count += 1;
        }

        debug!("slabinfo: updated {} slabs", slab_count);
        Ok(())
    }
}

impl Monitor for SlabInfoMonitor {
    fn name(&self) -> &'static &str {
        &"slabinfo"
    }

    fn collect(&mut self) -> Result<()> {
        self.collect_once()
    }
}
