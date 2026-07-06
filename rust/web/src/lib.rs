#![recursion_limit = "512"]

pub mod app;

#[cfg(feature = "ssr")]
pub mod state;

#[cfg(feature = "ssr")]
pub mod router;

#[cfg(feature = "ssr")]
pub mod db;

#[cfg(feature = "ssr")]
pub mod nats;

pub mod websocket;

#[cfg(feature = "ssr")]
pub mod models;

pub mod auth;

pub mod game;

pub mod websocket_client;

pub mod components;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
    // Flag hydration as complete so e2e tests can wait for a definitive
    // signal instead of guessing from network activity (`networkidle` fires
    // as soon as the WASM module finishes downloading, which can be before
    // it has finished instantiating and attaching event listeners).
    if let Some(body) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.body())
    {
        let _ = body.dataset().set("hydrated", "true");
    }
}
