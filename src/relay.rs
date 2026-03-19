//! Relay data model — mirrors the Onionoo `/details` response.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct OnionooResponse {
    pub relays: Vec<Relay>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Relay {
    pub nickname:     Option<String>,
    pub fingerprint:  String,
    pub flags:        Option<Vec<String>>,
    pub or_addresses: Option<Vec<String>>,
    pub latitude:     Option<f64>,
    pub longitude:    Option<f64>,
    pub country:      Option<String>,
    pub country_name: Option<String>,
    pub as_name:      Option<String>,
    pub bandwidth_rate:     Option<u64>,
    pub observed_bandwidth: Option<u64>,
    pub platform:     Option<String>,
    pub contact:      Option<String>,
    pub exit_policy:  Option<Vec<String>>,
    pub first_seen:   Option<String>,
    pub last_seen:    Option<String>,
}

impl Relay {
    pub fn flags(&self) -> &[String] {
        self.flags.as_deref().unwrap_or(&[])
    }

    pub fn has_flag(&self, f: &str) -> bool {
        self.flags().iter().any(|s| s.eq_ignore_ascii_case(f))
    }

    pub fn is_guard(&self)  -> bool { self.has_flag("Guard")  }
    pub fn is_exit(&self)   -> bool { self.has_flag("Exit")   }
    pub fn is_stable(&self) -> bool { self.has_flag("Stable") }
    pub fn is_fast(&self)   -> bool { self.has_flag("Fast")   }

    pub fn relay_type(&self) -> RelayType {
        match (self.is_guard(), self.is_exit()) {
            (true,  true)  => RelayType::GuardExit,
            (true,  false) => RelayType::Guard,
            (false, true)  => RelayType::Exit,
            (false, false) => RelayType::Middle,
        }
    }

    pub fn primary_ip(&self) -> Option<String> {
        let addr = self.or_addresses.as_deref()?.first()?.clone();
        // Strip port — IPv4: "1.2.3.4:9001" | IPv6: "[dead::1]:443"
        if addr.starts_with('[') {
            addr.split(']').next().map(|s| s.trim_start_matches('[').to_string())
        } else {
            addr.rsplit_once(':').map(|(ip, _)| ip.to_string())
        }
    }

    pub fn display_name(&self) -> String {
        self.nickname
            .clone()
            .unwrap_or_else(|| self.fingerprint[..8].to_string())
    }

    pub fn bandwidth_mbs(&self) -> Option<f64> {
        self.observed_bandwidth.map(|b| b as f64 / 1_000_000.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayType {
    Guard,
    Exit,
    GuardExit,
    Middle,
}

impl RelayType {
    pub fn label(self) -> &'static str {
        match self {
            RelayType::Guard     => "Guard",
            RelayType::Exit      => "Exit",
            RelayType::GuardExit => "Guard+Exit",
            RelayType::Middle    => "Middle",
        }
    }

    pub fn color(self) -> egui::Color32 {
        match self {
            RelayType::Guard     => egui::Color32::from_rgb(192, 132, 252), // purple
            RelayType::Exit      => egui::Color32::from_rgb(248, 113, 113), // red
            RelayType::GuardExit => egui::Color32::from_rgb(251, 191,  36), // amber
            RelayType::Middle    => egui::Color32::from_rgb(94,  234, 212), // teal
        }
    }

    pub fn dot_radius(self) -> f32 {
        match self {
            RelayType::Guard | RelayType::Exit | RelayType::GuardExit => 7.0,
            RelayType::Middle => 5.0,
        }
    }
}
