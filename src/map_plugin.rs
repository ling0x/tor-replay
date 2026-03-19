//! Walkers map plugin — draws Tor relay dots (or cluster bubbles) and
//! handles click / hover.

use std::sync::Arc;

use egui::{Color32, Response, Stroke, Ui};
use walkers::{MapMemory, Plugin, Projector, lat_lon};

use crate::cluster::{self, CLUSTER_ZOOM_THRESHOLD};
use crate::relay::{OnionooResponse, Relay, RelayType};

// ── Per-frame plugin ─────────────────────────────────────────────────────────

pub struct RelayMapPlugin<'a> {
    pub relays:        Arc<OnionooResponse>,
    pub hovered:       &'a mut Option<usize>,
    pub selected:      &'a mut Option<usize>,
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

        relay.position().is_some()
    }
}

impl<'a> Plugin for RelayMapPlugin<'a> {
    fn run(
        self: Box<Self>,
        ui: &mut Ui,
        response: &Response,
        projector: &Projector,
        map_memory: &MapMemory,
    ) {
        let zoom = map_memory.zoom();

        if zoom < CLUSTER_ZOOM_THRESHOLD {
            self.run_clustered(ui, response, projector);
        } else {
            self.run_individual(ui, response, projector);
        }
    }
}

impl<'a> RelayMapPlugin<'a> {
    fn run_clustered(
        self: Box<Self>,
        ui: &mut Ui,
        response: &Response,
        projector: &Projector,
    ) {
        let hover_pos = response.hover_pos();

        let clusters = cluster::build_clusters(
            self.relays.relays.iter(),
            self.filter_guard,
            self.filter_exit,
            self.filter_middle,
        );

        for c in &clusters {
            let pos = projector.project(lat_lon(c.lat, c.lon)).to_pos2();
            let r   = c.radius();
            let color = c.dominant_type().color();

            let hovered = hover_pos
                .map(|hp| hp.distance(pos) < r + 4.0)
                .unwrap_or(false);

            // Outer glow
            ui.painter().circle_filled(
                pos, r + 2.0,
                color.gamma_multiply(0.18),
            );
            // Main bubble
            ui.painter().circle_filled(
                pos, r,
                color.gamma_multiply(if hovered { 0.75 } else { 0.55 }),
            );
            ui.painter().circle_stroke(
                pos, r,
                Stroke::new(1.2, color.gamma_multiply(0.9)),
            );
            // Count label
            ui.painter().text(
                pos,
                egui::Align2::CENTER_CENTER,
                format!("{}", c.count),
                egui::FontId::proportional(if r > 20.0 { 11.0 } else { 9.0 }),
                Color32::WHITE,
            );

            if hovered {
                let tooltip_pos = pos + egui::vec2(r + 6.0, -12.0);
                #[allow(deprecated)]
                egui::show_tooltip_at(
                    ui.ctx(),
                    ui.layer_id(),
                    egui::Id::new("cluster_tooltip").with(
                        (c.lat as i32, c.lon as i32)
                    ),
                    tooltip_pos,
                    |ui| {
                        ui.set_max_width(180.0);
                        ui.strong(format!("{} relays", c.count));
                        ui.horizontal(|ui| {
                            dot(ui, RelayType::Guard.color());
                            ui.label(
                                egui::RichText::new(format!("{} guards", c.n_guard)).small(),
                            );
                        });
                        ui.horizontal(|ui| {
                            dot(ui, RelayType::Exit.color());
                            ui.label(
                                egui::RichText::new(format!("{} exits", c.n_exit)).small(),
                            );
                        });
                        ui.horizontal(|ui| {
                            dot(ui, RelayType::Middle.color());
                            ui.label(
                                egui::RichText::new(format!("{} middles", c.n_middle)).small(),
                            );
                        });
                        ui.label(
                            egui::RichText::new("zoom in to inspect").weak().italics().small(),
                        );
                    },
                );
            }
        }
    }

    fn run_individual(
        self: Box<Self>,
        ui: &mut Ui,
        response: &Response,
        projector: &Projector,
    ) {
        let hover_pos = response.hover_pos();
        let click_pos = if response.clicked_by(egui::PointerButton::Primary) {
            response.interact_pointer_pos()
        } else {
            None
        };

        // Two-pass: middles first, then guards/exits on top
        for pass in [false, true] {
            for (idx, relay) in self.relays.relays.iter().enumerate() {
                let notable = matches!(
                    relay.relay_type(),
                    RelayType::Guard | RelayType::Exit | RelayType::GuardExit
                );
                if notable != pass { continue; }
                if !self.should_show(relay) { continue; }

                let (lat, lon) = match relay.position() {
                    Some(p) => p,
                    None    => continue,
                };
                let pos = projector.project(lat_lon(lat, lon)).to_pos2();

                let rt     = relay.relay_type();
                let base_r = rt.dot_radius();
                let color  = rt.color();

                let hovered  = hover_pos.map(|hp| hp.distance(pos) < base_r + 4.0).unwrap_or(false);
                let selected = *self.selected == Some(idx);

                if let Some(cp) = click_pos {
                    if cp.distance(pos) < base_r + 4.0 {
                        *self.selected = Some(idx);
                    }
                }
                if hovered { *self.hovered = Some(idx); }

                let r = if hovered || selected { base_r + 3.0 } else { base_r };

                if selected {
                    ui.painter().circle_stroke(
                        pos, r + 4.0,
                        Stroke::new(1.5, color.gamma_multiply(0.5)),
                    );
                }

                ui.painter().circle_filled(
                    pos, r,
                    color.gamma_multiply(if hovered { 1.0 } else { 0.85 }),
                );
                ui.painter().circle_stroke(
                    pos, r,
                    Stroke::new(0.8, Color32::from_black_alpha(120)),
                );

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
                                    6.0, color,
                                );
                                ui.add_space(16.0);
                                ui.strong(relay.display_name());
                            });
                            if let Some(ip) = relay.primary_ip() {
                                ui.label(egui::RichText::new(ip).monospace().small());
                            }
                            ui.label(egui::RichText::new(rt.label()).small().color(color));
                            if let Some(cc) = &relay.country_name {
                                ui.label(egui::RichText::new(cc).small());
                            }
                            ui.label(
                                egui::RichText::new("click to inspect").weak().italics().small(),
                            );
                        },
                    );
                }
            }
        }
    }
}

fn dot(ui: &mut Ui, color: Color32) {
    let (resp, painter) = ui.allocate_painter(egui::vec2(12.0, 12.0), egui::Sense::hover());
    painter.circle_filled(resp.rect.center(), 5.0, color);
}
