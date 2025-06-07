pub mod app;

#[cfg(feature = "ssr")]
pub mod db;

#[cfg(feature = "ssr")]
pub mod models;

pub mod auth;

pub mod game;

pub mod components;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
