//! #22b play-by-email module: turn notifications, reminders, and reply
//! handling. SSR-only - never compiled into the WASM bundle.
pub mod commands;
pub mod inbound;
pub mod notify;
pub mod outbound;
pub mod render;
