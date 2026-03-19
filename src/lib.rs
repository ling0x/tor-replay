mod app;
mod fetch;
mod map_plugin;
mod relay;

pub use app::TorReplayApp;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let document = web_sys::window()
        .ok_or("no window")?
        .document()
        .ok_or("no document")?;

    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| format!("canvas #{canvas_id} not found"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "element is not a canvas")?;

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async move {
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(TorReplayApp::new(cc)))),
            )
            .await;

        if let Err(e) = result {
            web_sys::console::error_1(
                &format!("eframe::WebRunner::start error: {e:?}").into(),
            );
        }
    });

    Ok(())
}
