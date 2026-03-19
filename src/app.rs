//! Main eframe application — layout, panels, state.

use std::sync::{Arc, Mutex};

use egui::{
    CentralPanel, Color32, Context, Frame, RichText, ScrollArea, SidePanel, TopBottomPanel,
    Vec2, Stroke,
};
use walkers::{HttpTiles, Map, MapMemory, lat_lon, sources::OpenStreetMap};

use crate::{
    fetch::{self, FetchState, SharedState},
    map_plugin::RelayMapPlugin,
    relay::{OnionooResponse, Relay, RelayType},
};

// ---------------------------------------------------------------------------
// Colour constants
// ---------------------------------------------------------------------------
const BG:      Color32 = Color32::from_rgb( 10,  18,  35);
const SURFACE: Color32 = Color32::from_rgb( 15,  27,  52);
const BORDER:  Color32 = Color32::from_rgb( 30,  50,  90);
const TEXT_DIM:Color32 = Color32::from_rgb(100, 130, 170);
const ACCENT:  Color32 = Color32::from_rgb( 94, 234, 212); // teal

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

pub struct TorReplayApp {
    tiles:        HttpTiles,
    map_memory:   MapMemory,
    fetch_state:  SharedState,

    // Filters
    filter_guard:  bool,
    filter_exit:   bool,
    filter_middle: bool,
    search_query:  String,

    // Interaction
    hovered_idx:  Option<usize>,
    selected_idx: Option<usize>,

    // Stats cache (recomputed when data arrives)
    total_relays: usize,
    n_guard:      usize,
    n_exit:       usize,
    n_middle:     usize,
    n_countries:  usize,

    show_about: bool,
}

impl TorReplayApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        configure_visuals(&cc.egui_ctx);

        let tiles = HttpTiles::new(OpenStreetMap, cc.egui_ctx.clone());

        let fetch_state: SharedState = Arc::new(Mutex::new(FetchState::Idle));
        // Kick off the live Onionoo fetch
        fetch::start_fetch(Arc::clone(&fetch_state), cc.egui_ctx.clone());

        // Start zoomed out to show the whole world
        let mut map_memory = MapMemory::default();
        let _ = map_memory.set_zoom(2.0);

        Self {
            tiles,
            map_memory,
            fetch_state,
            filter_guard:  true,
            filter_exit:   true,
            filter_middle: true,
            search_query:  String::new(),
            hovered_idx:   None,
            selected_idx:  None,
            total_relays: 0,
            n_guard:      0,
            n_exit:       0,
            n_middle:     0,
            n_countries:  0,
            show_about:   false,
        }
    }

    fn data(&self) -> Option<Arc<OnionooResponse>> {
        match &*self.fetch_state.lock().unwrap() {
            FetchState::Done(d) => Some(Arc::clone(d)),
            _ => None,
        }
    }

    fn update_stats(&mut self, data: &OnionooResponse) {
        self.total_relays = data.relays.len();
        self.n_guard  = data.relays.iter().filter(|r| r.is_guard() && !r.is_exit()).count();
        self.n_exit   = data.relays.iter().filter(|r| r.is_exit()).count();
        self.n_middle = data.relays.iter()
            .filter(|r| !r.is_guard() && !r.is_exit()).count();
        use std::collections::HashSet;
        let ccs: HashSet<_> = data.relays.iter()
            .filter_map(|r| r.country.as_deref())
            .collect();
        self.n_countries = ccs.len();
    }
}

impl eframe::App for TorReplayApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Sync stats whenever data arrives
        if let Some(data) = self.data() {
            if self.total_relays == 0 {
                self.update_stats(&data);
            }
        }

        self.hovered_idx = None; // reset each frame

        draw_top_bar(self, ctx);
        draw_left_panel(self, ctx);
        draw_right_panel(self, ctx);
        draw_map(self, ctx);

        if self.show_about { draw_about_window(self, ctx); }
    }
}

// ---------------------------------------------------------------------------
// Top bar
// ---------------------------------------------------------------------------
fn draw_top_bar(app: &mut TorReplayApp, ctx: &Context) {
    TopBottomPanel::top("topbar")
        .frame(Frame {
            fill: SURFACE,
            inner_margin: egui::Margin::symmetric(16, 8),
            stroke: Stroke::new(1.0, BORDER),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Logo / title
                ui.add_space(4.0);
                tor_logo(ui, 28.0);
                ui.add_space(8.0);
                ui.label(
                    RichText::new("tor-replay")
                        .size(20.0)
                        .strong()
                        .color(ACCENT),
                );
                ui.label(
                    RichText::new("live relay dashboard")
                        .size(12.0)
                        .color(TEXT_DIM),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("ℹ about").clicked() {
                        app.show_about = !app.show_about;
                    }
                    ui.separator();
                    // Fetch status badge
                    let state = app.fetch_state.lock().unwrap().clone();
                    match state {
                        FetchState::Loading => {
                            ui.spinner();
                            ui.label(RichText::new("fetching Onionoo…").color(TEXT_DIM).small());
                        }
                        FetchState::Done(_) => {
                            ui.label(
                                RichText::new(format!("● {} relays", app.total_relays))
                                    .color(Color32::from_rgb(74, 222, 128))
                                    .small(),
                            );
                        }
                        FetchState::Error(ref e) => {
                            ui.label(RichText::new(format!("✗ {}", &e[..e.len().min(40)])).color(Color32::RED).small());
                        }
                        FetchState::Idle => {}
                    }
                    ui.separator();
                    // Search
                    ui.add(
                        egui::TextEdit::singleline(&mut app.search_query)
                            .hint_text("🔍 search relay / IP / fingerprint")
                            .desired_width(220.0),
                    );
                });
            });
        });
}

// ---------------------------------------------------------------------------
// Left panel — filters + stats
// ---------------------------------------------------------------------------
fn draw_left_panel(app: &mut TorReplayApp, ctx: &Context) {
    SidePanel::left("left_panel")
        .resizable(true)
        .default_width(200.0)
        .min_width(160.0)
        .max_width(280.0)
        .frame(Frame {
            fill: SURFACE,
            inner_margin: egui::Margin::same(12),
            stroke: Stroke::new(1.0, BORDER),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.heading(RichText::new("NETWORK").color(ACCENT).size(11.0));
            ui.add_space(8.0);

            // Stats grid
            stat_row(ui, "Total relays",  &format!("{}", app.total_relays), ACCENT);
            stat_row(ui, "Countries",     &format!("{}", app.n_countries),  TEXT_DIM);
            ui.add_space(6.0);
            stat_row(ui, "Guards",  &format!("{}", app.n_guard),  RelayType::Guard.color());
            stat_row(ui, "Exits",   &format!("{}", app.n_exit),   RelayType::Exit.color());
            stat_row(ui, "Middles", &format!("{}", app.n_middle), RelayType::Middle.color());

            ui.separator();
            ui.heading(RichText::new("FILTERS").color(ACCENT).size(11.0));
            ui.add_space(6.0);

            filter_toggle(ui, "● Guards",  RelayType::Guard.color(),  &mut app.filter_guard);
            filter_toggle(ui, "● Exits",   RelayType::Exit.color(),   &mut app.filter_exit);
            filter_toggle(ui, "● Middles", RelayType::Middle.color(), &mut app.filter_middle);

            ui.separator();
            ui.heading(RichText::new("MAP").color(ACCENT).size(11.0));
            ui.add_space(4.0);

            if ui.button("🌍 Reset view").clicked() {
                app.map_memory = MapMemory::default();
                let _ = app.map_memory.set_zoom(2.0);
            }
            if ui.button("✕ Clear selection").clicked() {
                app.selected_idx = None;
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.hyperlink_to(
                    RichText::new("Created with Perplexity Computer").weak().small(),
                    "https://www.perplexity.ai/computer",
                );
                ui.separator();
                ui.hyperlink_to(
                    RichText::new("Onionoo API").color(TEXT_DIM).small(),
                    "https://onionoo.torproject.org/",
                );
                ui.hyperlink_to(
                    RichText::new("Tor Project").color(TEXT_DIM).small(),
                    "https://www.torproject.org/",
                );
            });
        });
}

// ---------------------------------------------------------------------------
// Right panel — relay detail / relay list
// ---------------------------------------------------------------------------
fn draw_right_panel(app: &mut TorReplayApp, ctx: &Context) {
    SidePanel::right("right_panel")
        .resizable(true)
        .default_width(260.0)
        .min_width(200.0)
        .max_width(360.0)
        .frame(Frame {
            fill: SURFACE,
            inner_margin: egui::Margin::same(12),
            stroke: Stroke::new(1.0, BORDER),
            ..Default::default()
        })
        .show(ctx, |ui| {
            let data = app.data();

            if let Some(ref d) = data {
                // If a relay is selected, show its details
                if let Some(idx) = app.selected_idx {
                    if let Some(relay) = d.relays.get(idx) {
                        relay_detail_panel(ui, relay, &mut app.selected_idx);
                        return;
                    }
                }

                // Otherwise show relay list
                ui.heading(RichText::new("RELAY LIST").color(ACCENT).size(11.0));
                ui.add_space(4.0);
                ui.label(RichText::new("click a relay on the map or below").color(TEXT_DIM).small());
                ui.add_space(6.0);

                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for (idx, relay) in d.relays.iter().enumerate() {
                            if relay.position().is_none() { continue; }

                            let rt    = relay.relay_type();
                            let color = rt.color();
                            let name  = relay.display_name();
                            let ip    = relay.primary_ip().unwrap_or_default();

                            let is_selected = app.selected_idx == Some(idx);
                            let resp = ui.selectable_label(
                                is_selected,
                                RichText::new(format!("● {name}")).color(color).small(),
                            );
                            if resp.hovered() {
                                ui.painter().rect_stroke(
                                    resp.rect.expand(1.0),
                                    2.0,
                                    Stroke::new(1.0, color.gamma_multiply(0.5)),
                                    egui::StrokeKind::Outside,
                                );
                            }
                            if resp.clicked() {
                                app.selected_idx = Some(idx);
                            }
                            resp.on_hover_ui(|ui| {
                                ui.label(RichText::new(&ip).monospace().small());
                                ui.label(RichText::new(rt.label()).small().color(color));
                            });
                        }
                    });
            } else {
                let state = app.fetch_state.lock().unwrap().clone();
                match state {
                    FetchState::Loading => {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            ui.spinner();
                            ui.add_space(8.0);
                            ui.label(RichText::new("Loading relays from\nOnionoo…").color(TEXT_DIM).small());
                        });
                    }
                    FetchState::Error(e) => {
                        ui.colored_label(Color32::RED, format!("Error: {e}"));
                    }
                    _ => {
                        ui.label(RichText::new("No data yet").color(TEXT_DIM));
                    }
                }
            }
        });
}

// ---------------------------------------------------------------------------
// Map
// ---------------------------------------------------------------------------
fn draw_map(app: &mut TorReplayApp, ctx: &Context) {
    CentralPanel::default()
        .frame(Frame { fill: BG, ..Default::default() })
        .show(ctx, |ui| {
            let initial_pos = lat_lon(20.0, 0.0); // centred on globe

            // Pre-extract data before borrowing tiles/memory
            let relay_data = app.data();

            let mut map = Map::new(
                Some(&mut app.tiles),
                &mut app.map_memory,
                initial_pos,
            );

            if let Some(data) = relay_data {
                let plugin = RelayMapPlugin {
                    relays:        Arc::clone(&data),
                    hovered:       &mut app.hovered_idx,
                    selected:      &mut app.selected_idx,
                    filter_guard:  app.filter_guard,
                    filter_exit:   app.filter_exit,
                    filter_middle: app.filter_middle,
                    search_query:  app.search_query.clone(),
                };
                map = map.with_plugin(plugin);
            }

            map.show(ui, |_, _, _, _| ());

            // Map legend overlay
            draw_legend(ui);

            // Loading overlay
            {
                let state = app.fetch_state.lock().unwrap().clone();
                if matches!(state, FetchState::Loading) {
                    let rect = ui.clip_rect();
                    let center = rect.center() + Vec2::new(0.0, 20.0);
                    ui.painter().rect_filled(
                        egui::Rect::from_center_size(center, Vec2::new(220.0, 54.0)),
                        8.0,
                        Color32::from_black_alpha(180),
                    );
                    ui.painter().text(
                        center - Vec2::new(0.0, 8.0),
                        egui::Align2::CENTER_CENTER,
                        "Fetching live relay data…",
                        egui::FontId::proportional(14.0),
                        Color32::WHITE,
                    );
                    ui.painter().text(
                        center + Vec2::new(0.0, 10.0),
                        egui::Align2::CENTER_CENTER,
                        "onionoo.torproject.org",
                        egui::FontId::monospace(11.0),
                        TEXT_DIM,
                    );
                }
            }
        });
}

fn draw_legend(ui: &mut egui::Ui) {
    let rect = ui.clip_rect();
    let x0   = rect.min.x + 12.0;
    let y0   = rect.max.y - 80.0;

    let types = [
        ("Guard",  RelayType::Guard.color()),
        ("Exit",   RelayType::Exit.color()),
        ("Middle", RelayType::Middle.color()),
    ];

    ui.painter().rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(x0 - 8.0, y0 - 8.0),
            egui::vec2(100.0, 72.0),
        ),
        6.0,
        Color32::from_black_alpha(160),
    );

    for (i, (label, color)) in types.iter().enumerate() {
        let y = y0 + i as f32 * 20.0;
        ui.painter().circle_filled(egui::pos2(x0 + 6.0, y + 6.0), 5.0, *color);
        ui.painter().text(
            egui::pos2(x0 + 16.0, y + 6.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        );
    }
}

// ---------------------------------------------------------------------------
// Relay detail panel
// ---------------------------------------------------------------------------
fn relay_detail_panel(
    ui:       &mut egui::Ui,
    relay:    &Relay,
    selected: &mut Option<usize>,
) {
    let rt    = relay.relay_type();
    let color = rt.color();

    ui.horizontal(|ui| {
        ui.heading(RichText::new("RELAY DETAIL").color(ACCENT).size(11.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("✕").clicked() { *selected = None; }
        });
    });
    ui.separator();

    // Type badge
    ui.horizontal(|ui| {
        ui.painter().circle_filled(
            ui.cursor().min + egui::vec2(8.0, 10.0),
            8.0,
            color,
        );
        ui.add_space(20.0);
        ui.label(RichText::new(relay.display_name()).strong().size(15.0));
    });
    ui.add_space(2.0);
    ui.label(RichText::new(rt.label()).color(color).size(12.0));

    ui.separator();

    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            let mut detail = |label: &str, val: &str| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(label).color(TEXT_DIM).small());
                    ui.label(RichText::new(val).monospace().small());
                });
            };

            if let Some(ip) = relay.primary_ip() {
                detail("IP Address:", &ip);
            }
            if let Some(addrs) = &relay.or_addresses {
                for a in addrs {
                    detail("OR Address:", a);
                }
            }
            detail("Fingerprint:", &relay.fingerprint);
            if let Some(cc) = &relay.country_name {
                detail("Country:", cc);
            }
            if let Some(asn) = &relay.as_name {
                detail("AS Name:", asn);
            }
            if let Some(lat) = relay.latitude {
                if let Some(lon) = relay.longitude {
                    detail("Coordinates:", &format!("{lat:.4}, {lon:.4}"));
                }
            }
            if let Some(bw) = relay.bandwidth_mbs() {
                detail("Bandwidth:", &format!("{:.2} MB/s", bw));
            }
            if let Some(platform) = &relay.platform {
                detail("Platform:", &platform[..platform.len().min(60)]);
            }
            if let Some(fs) = &relay.first_seen {
                detail("First seen:", fs);
            }
            if let Some(ls) = &relay.last_seen {
                detail("Last seen:", ls);
            }
            if !relay.flags().is_empty() {
                detail("Flags:", &relay.flags().join(", "));
            }
            if let Some(contact) = &relay.contact {
                let trimmed = &contact[..contact.len().min(80)];
                detail("Contact:", trimmed);
            }
        });
}

// ---------------------------------------------------------------------------
// About window
// ---------------------------------------------------------------------------
fn draw_about_window(app: &mut TorReplayApp, ctx: &Context) {
    egui::Window::new("About tor-replay")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(380.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                tor_logo(ui, 48.0);
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.heading("tor-replay");
                    ui.label(RichText::new("Live Tor relay world map").color(TEXT_DIM));
                });
            });
            ui.separator();
            ui.label("Built with egui + walkers (OpenStreetMap) + Onionoo API.");
            ui.add_space(4.0);
            ui.label("Relay data is fetched live from the Tor Project's Onionoo service.");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.hyperlink_to("GitHub → ling0x/tor-replay", "https://github.com/ling0x/tor-replay");
            });
            ui.horizontal(|ui| {
                ui.hyperlink_to("Onionoo API docs", "https://metrics.torproject.org/onionoo.html");
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.hyperlink_to(
                    RichText::new("Created with Perplexity Computer").weak().small(),
                    "https://www.perplexity.ai/computer",
                );
            });
            ui.add_space(4.0);
            if ui.button("Close").clicked() {
                app.show_about = false;
            }
        });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
fn stat_row(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(TEXT_DIM).small());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).strong().color(color).small());
        });
    });
}

fn filter_toggle(ui: &mut egui::Ui, label: &str, color: Color32, val: &mut bool) {
    ui.horizontal(|ui| {
        let resp = ui.selectable_label(*val, RichText::new(label).color(color).small());
        if resp.clicked() { *val = !*val; }
    });
}

fn tor_logo(ui: &mut egui::Ui, size: f32) {
    // Minimal onion SVG — drawn inline using egui painter
    let (resp, painter) = ui.allocate_painter(Vec2::splat(size), egui::Sense::hover());
    let rect = resp.rect;
    let c    = rect.center();
    let r    = size * 0.42;

    // Three concentric rings (onion layers)
    for (i, alpha) in [(0, 255u8), (1, 160), (2, 80)].iter() {
        let layer_r = r - *i as f32 * (r / 3.6);
        painter.circle_stroke(
            c,
            layer_r,
            Stroke::new(size * 0.06, ACCENT.linear_multiply(*alpha as f32 / 255.0)),
        );
    }
    // Centre dot
    painter.circle_filled(c, size * 0.08, ACCENT);
}

// ---------------------------------------------------------------------------
// Visuals
// ---------------------------------------------------------------------------
fn configure_visuals(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill          = SURFACE;
    visuals.window_fill         = SURFACE;
    visuals.extreme_bg_color    = BG;
    visuals.faint_bg_color      = Color32::from_rgb(18, 32, 58);
    visuals.code_bg_color       = Color32::from_rgb(12, 22, 42);
    visuals.override_text_color = Some(Color32::from_rgb(220, 230, 245));
    visuals.window_stroke       = Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.selection.bg_fill   = ACCENT.gamma_multiply(0.25);
    visuals.selection.stroke    = Stroke::new(1.0, ACCENT);

    ctx.set_visuals(visuals);

    // Font sizes
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(13.0),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional(11.0),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(16.0),
    );
    ctx.set_style(style);
}
