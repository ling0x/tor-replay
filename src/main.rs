// Native desktop entry-point. WASM builds use src/lib.rs instead.
// This file is unconditionally compiled but all the real code is cfg-guarded.

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    run_native();
}

#[cfg(not(target_arch = "wasm32"))]
fn run_native() {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("tor-replay")
            .with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "tor-replay",
        native_options,
        Box::new(|cc| Ok(Box::new(tor_replay::TorReplayApp::new(cc)))),
    )
    .unwrap();
}
