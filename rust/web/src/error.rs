use leptos::prelude::ServerFnError;

/// For `.map_err(...)` on infrastructure failures inside server functions:
/// logs the real error server-side and replaces it with an opaque message,
/// so database/service internals never reach the client.
#[cfg(feature = "ssr")]
pub fn internal<E: std::fmt::Display>(context: &'static str) -> impl FnOnce(E) -> ServerFnError {
    move |e| {
        tracing::error!("{}: {}", context, e);
        ServerFnError::new("Internal server error")
    }
}

pub fn user_facing_server_error(_e: &ServerFnError) -> String {
    "Something went wrong, please try again".to_string()
}
