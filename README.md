# tor-replay

**Live Tor relay world map dashboard** — built with [egui](https://github.com/emilk/egui), [walkers](https://github.com/podusowski/walkers), and WebAssembly.

![Dashboard showing world map with Tor relay dots](https://raw.githubusercontent.com/ling0x/tor-replay/main/screenshot.png)

## Features

- 🌍 **Interactive world map** — pan, zoom, powered by OpenStreetMap tiles via walkers
- 🟣 **Live relay data** — fetched from [Onionoo](https://onionoo.torproject.org/) on load (~8,000 relays)
- 🖱️ **Hover tooltips** — hover any relay dot to see its IP, country, relay type
- 🖱️ **Click to inspect** — click a relay for full details (fingerprint, AS name, bandwidth, flags, contact)
- 🔍 **Search** — filter by nickname, IP address, or fingerprint
- 🎛️ **Type filter** — toggle Guards (purple), Exits (red), Middles (teal) independently
- 📊 **Stats sidebar** — live counts of guards, exits, middles, country count

## Relay Colour Coding

| Colour | Type |
|--------|------|
| 🟣 Purple | Guard relay |
| 🔴 Red | Exit relay |
| 🟡 Amber | Guard+Exit relay |
| 🩵 Teal | Middle relay |

## Running locally

```bash
# Install deps
rustup target add wasm32-unknown-unknown
cargo install --locked trunk

# Dev server (hot-reload)
trunk serve

# Release build
trunk build --release
# Outputs to ./dist/
```

## Data source

Relay metadata is fetched live from [Onionoo](https://onionoo.torproject.org/details) — the Tor Project's relay information service.

## Stack

- [`eframe`](https://crates.io/crates/eframe) / [`egui`](https://crates.io/crates/egui) 0.33 — immediate-mode GUI, WebGL backend
- [`walkers`](https://crates.io/crates/walkers) 0.52 — slippy map widget (OSM tiles)
- [`reqwest`](https://crates.io/crates/reqwest) — async HTTP, WASM-compatible
- [`trunk`](https://trunkrs.dev/) — WASM bundler
- [`wasm-bindgen`](https://crates.io/crates/wasm-bindgen) — JS/WASM bindings

---
Built with [Perplexity Computer](https://www.perplexity.ai/computer)
