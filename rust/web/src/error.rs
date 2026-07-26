use leptos::prelude::ServerFnError;

/// The single opaque message `internal` substitutes for an infrastructure
/// failure. Named so callers that need to tell "a redacted internal failure"
/// apart from "a deliberate user-facing message" can compare against it
/// instead of a magic literal (see `email::commands::classify_server_fn_error`).
pub const INTERNAL_ERROR_MESSAGE: &str = "Internal server error";

/// For `.map_err(...)` on infrastructure failures inside server functions:
/// logs the real error server-side and replaces it with an opaque message,
/// so database/service internals never reach the client.
#[cfg(feature = "ssr")]
pub fn internal<E: std::fmt::Display>(context: &'static str) -> impl FnOnce(E) -> ServerFnError {
    move |e| {
        tracing::error!("{}: {}", context, e);
        ServerFnError::new(INTERNAL_ERROR_MESSAGE)
    }
}

pub fn user_facing_server_error(_e: &ServerFnError) -> String {
    "Something went wrong, please try again".to_string()
}
