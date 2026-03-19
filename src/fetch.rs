//! Async fetch logic — platform-aware (WASM / native via reqwest).

use std::sync::{Arc, Mutex};

use crate::relay::OnionooResponse;

const ONIONOO_URL: &str =
    "https://onionoo.torproject.org/details?search=type:relay%20running:true\
     &fields=nickname,fingerprint,flags,or_addresses,latitude,longitude,\
     country,country_name,as_name,bandwidth_rate,observed_bandwidth,\
     platform,contact,first_seen,last_seen";

#[derive(Debug, Clone)]
pub enum FetchState {
    Idle,
    Loading,
    Done(Arc<OnionooResponse>),
    Error(String),
}

impl Default for FetchState {
    fn default() -> Self { FetchState::Idle }
}

/// Shared, interior-mutable state updated from an async task.
pub type SharedState = Arc<Mutex<FetchState>>;

/// Kick off a fetch.  The result is written into `state`.
pub fn start_fetch(state: SharedState, ctx: egui::Context) {
    {
        let mut s = state.lock().unwrap();
        *s = FetchState::Loading;
    }

    let state_clone = Arc::clone(&state);
    let ctx_clone   = ctx.clone();

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_onionoo().await {
                Ok(resp) => {
                    let mut s = state_clone.lock().unwrap();
                    *s = FetchState::Done(Arc::new(resp));
                }
                Err(e) => {
                    let mut s = state_clone.lock().unwrap();
                    *s = FetchState::Error(e);
                }
            }
            ctx_clone.request_repaint();
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            match rt.block_on(fetch_onionoo()) {
                Ok(resp) => {
                    let mut s = state_clone.lock().unwrap();
                    *s = FetchState::Done(Arc::new(resp));
                }
                Err(e) => {
                    let mut s = state_clone.lock().unwrap();
                    *s = FetchState::Error(e);
                }
            }
            ctx_clone.request_repaint();
        });
    }
}

async fn fetch_onionoo() -> Result<OnionooResponse, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(ONIONOO_URL)
        .header("User-Agent", "tor-replay/0.1 (github.com/ling0x/tor-replay)")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json::<OnionooResponse>()
        .await
        .map_err(|e| e.to_string())
}
