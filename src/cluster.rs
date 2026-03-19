//! Spatial clustering for relay dots.
//!
//! At low zoom levels we bucket relays into a grid of cells and render
//! one count-bubble per cell instead of thousands of individual dots.
//! Above CLUSTER_ZOOM_THRESHOLD we render individual dots as before.

use crate::relay::{Relay, RelayType};

pub const CLUSTER_ZOOM_THRESHOLD: f64 = 5.0;

/// Grid resolution in degrees. At zoom ~2 this makes ~20×10 cells.
const CELL_DEG: f64 = 10.0;

#[derive(Debug, Clone)]
pub struct Cluster {
    /// Centre of the cluster cell (lat, lon)
    pub lat: f64,
    pub lon: f64,
    pub count: usize,
    pub n_guard: usize,
    pub n_exit: usize,
    pub n_middle: usize,
}

impl Cluster {
    /// Dominant relay type for colouring the bubble.
    pub fn dominant_type(&self) -> RelayType {
        if self.n_exit >= self.n_guard && self.n_exit >= self.n_middle {
            RelayType::Exit
        } else if self.n_guard >= self.n_middle {
            RelayType::Guard
        } else {
            RelayType::Middle
        }
    }

    /// Bubble radius scaled by relay count.
    pub fn radius(&self) -> f32 {
        (6.0 + (self.count as f32).sqrt() * 1.8).min(38.0)
    }
}

/// Build clusters from a slice of relays at the given zoom level.
pub fn build_clusters<'a>(
    relays: impl Iterator<Item = &'a Relay>,
    filter_guard: bool,
    filter_exit: bool,
    filter_middle: bool,
) -> Vec<Cluster> {
    use std::collections::HashMap;

    // key = (lat_cell, lon_cell) in integer grid units
    let mut map: HashMap<(i32, i32), Cluster> = HashMap::new();

    for relay in relays {
        let (lat, lon) = match relay.position() {
            Some(p) => p,
            None => continue,
        };

        let rt = relay.relay_type();
        let visible = match rt {
            RelayType::Guard | RelayType::GuardExit => filter_guard,
            RelayType::Exit  => filter_exit,
            RelayType::Middle => filter_middle,
        };
        if !visible { continue; }

        let cell_lat = (lat / CELL_DEG).floor() as i32;
        let cell_lon = (lon / CELL_DEG).floor() as i32;

        let cluster = map.entry((cell_lat, cell_lon)).or_insert_with(|| {
            // Cell centre
            let clat = (cell_lat as f64 + 0.5) * CELL_DEG;
            let clon = (cell_lon as f64 + 0.5) * CELL_DEG;
            Cluster {
                lat: clat,
                lon: clon,
                count: 0,
                n_guard: 0,
                n_exit: 0,
                n_middle: 0,
            }
        });

        cluster.count += 1;
        match rt {
            RelayType::Guard | RelayType::GuardExit => cluster.n_guard += 1,
            RelayType::Exit   => cluster.n_exit   += 1,
            RelayType::Middle => cluster.n_middle += 1,
        }
    }

    map.into_values().collect()
}
