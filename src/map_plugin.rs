//! Walkers map plugin — draws Tor relay dots and handles click / hover.

use std::sync::Arc;

use egui::{Color32, Response, Ui};
use walkers::{MapMemory, Plugin, Projector, lat_lon};

use crate::relay::{OnionooResponse, Relay, RelayType};

/// State shared between the plugin (each frame) and the app.
pub struct RelayPlugin {
    pub relays:   Arc<OnionooResponse>,
    pub hovered:  Option<usize>,   // index into relays
    pub selected: Option<usize>,
    pub filter_guard:  bool,
    pub filter_exit:   bool,
    pub filter_middle: bool,
    pub search_query:  String,
}

impl RelayPlugin {
    fn should_show(&self, relay: &Relay) -> bool {
        let rt = relay.relay_type();
        let type_ok = match rt {
            RelayType::Guard | RelayType::GuardExit => self.filter_guard,
            RelayType::Exit                         => self.filter_exit,
            RelayType::Middle                       => self.filter_middle,
        };
        if !type_ok { return false; }

        if !self.search_query.is_empty() {
            let q = self.search_query.to_lowercase();
            let name_match = relay.display_name().to_lowercase().contains(&q);
            let ip_match   = relay.primary_ip()
                .map(|ip| ip.contains(&q))
                .unwrap_or(false);
            let fp_match   = relay.fingerprint.to_lowercase().contains(&q);
            if !name_match && !ip_match && !fp_match { return false; }
        }

        relay.latitude.is_some() && relay.longitude.is_some()
    }
}

/// Each frame we create a new plugin instance (egui immediate mode).
pub struct RelayMapPlugin<'a> {
    pub relays:   Arc<OnionooResponse>,
    pub hovered:  &'a mut Option<usize>,
    pub selected: &'a mut Option<usize>,
    pub filter_guard:  bool,
    pub filter_exit:   bool,
    pub filter_middle: bool,
    pub search_query:  String,
}

impl<'a> RelayMapPlugin<'a> {
    fn should_show(&self, relay: &Relay) -> bool {
        let rt = relay.relay_type();
        let type_ok = match rt {
            RelayType::Guard | RelayType::GuardExit => self.filter_guard,
            RelayType::Exit                         => self.filter_exit,
            RelayType::Middle                       => self.filter_middle,
        };
        if !type_ok { return false; }

        if !self.search_query.is_empty() {
            let q = self.search_query.to_lowercase();
            let nm = relay.display_name().to_lowercase().contains(&q);
            let im = relay.primary_ip().map(|ip| ip.contains(&q)).unwrap_or(false);
            let fm = relay.fingerprint.to_lowercase().contains(&q);
            if !nm && !im && !fm { return false; }
        }

        relay.latitude.is_some() && relay.longitude.is_some()
    }
}

impl<'a> Plugin for RelayMapPlugin<'a> {
    fn run(
        self: Box<Self>,
        ui: &mut Ui,
        response: &Response,
        projector: &Projector,
        _map_memory: &MapMemory,
    ) {
        let hover_pos = response.hover_pos();
        let click_pos = if response.clicked_by(egui::PointerButton::Primary) {
            response.interact_pointer_pos()
        } else {
            None
        };

        // Two-pass: middles first, then guards/exits on top.
        for pass in [false, true] {
            for (idx, relay) in self.relays.relays.iter().enumerate() {
                let notable = matches!(
                    relay.relay_type(),
                    RelayType::Guard | RelayType::Exit | RelayType::GuardExit
                );
                if notable != pass { continue; }
                if !self.should_show(relay) { continue; }

                let lat = relay.latitude.unwrap();
                let lon = relay.longitude.unwrap();
                let pos = projector.project(lat_lon(lat, lon)).to_pos2();

                let rt      = relay.relay_type();
                let base_r  = rt.dot_radius();
                let color   = rt.color();

                // Hover detection
                let hovered = hover_pos
                    .map(|hp| hp.distance(pos) < base_r + 4.0)
                    .unwrap_or(false);

                let selected = *self.selected == Some(idx);

                // Click detection
                if let Some(cp) = click_pos {
                    if cp.distance(pos) < base_r + 4.0 {
                        *self.selected = Some(idx);
                    }
                }

                // Update hovered index for tooltip
                if hovered {
                    *self.hovered = Some(idx);
                }

                let r = if hovered || selected { base_r + 3.0 } else { base_r };

                // Outer glow ring for selected
                if selected {
                    ui.painter().circle_stroke(
                        pos,
                        r + 4.0,
                        egui::Stroke::new(1.5, color.gamma_multiply(0.5)),
                    );
                }

                // Dot with outline
                ui.painter().circle_filled(
                    pos,
                    r,
                    color.gamma_multiply(if hovered { 1.0 } else { 0.85 }),
                );
                ui.painter().circle_stroke(
                    pos,
                    r,
                    egui::Stroke::new(0.8, Color32::from_black_alpha(120)),
                );

                // Hover tooltip — small popup near the dot
                if hovered {
                    let tooltip_pos = pos + egui::vec2(r + 6.0, -12.0);
                    #[allow(deprecated)]
                    egui::show_tooltip_at(
                        ui.ctx(),
                        ui.layer_id(),
                        egui::Id::new("relay_tooltip").with(idx),
                        tooltip_pos,
                        |ui| {
                            ui.set_max_width(240.0);
                            ui.horizontal(|ui| {
                                ui.painter().circle_filled(
                                    ui.cursor().min + egui::vec2(6.0, 8.0),
                                    6.0,
                                    color,
                                );
                                ui.add_space(16.0);
                                ui.strong(relay.display_name());
                            });
                            if let Some(ip) = relay.primary_ip() {
                                ui.label(egui::RichText::new(ip).monospace().small());
                            }
                            ui.label(
                                egui::RichText::new(rt.label())
                                    .small()
                                    .color(color),
                            );
                            if let Some(cc) = &relay.country_name {
                                ui.label(egui::RichText::new(cc).small());
                            }
                            ui.label(egui::RichText::new("click to inspect").weak().italics().small());
                        },
                    );
                }
            }
        }
    }
}
