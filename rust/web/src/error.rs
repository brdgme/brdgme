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

/// Message to show a user when a server-fn call returns `Err`.
///
/// `ServerFnError`'s `Display` impl prefixes every variant with framework
/// noise - `ServerError(s)` renders as "error running server function: {s}"
/// (server_fn-0.8.13/src/error.rs:233-234) - so `e.to_string()` must never
/// reach the UI. Server fns raise deliberate user-facing rejections with
/// `ServerFnError::new(msg)`, i.e. the `ServerError` variant, and
/// `internal()` above has already replaced genuine infrastructure failures
/// with the opaque "Internal server error" before they get here, so that
/// message is safe to show verbatim. Every other variant is transport or
/// (de)serialization and collapses to the generic text.
///
/// Use this for a failed *write* (a dispatched action). For a failed *read*
/// (a resource that would not load) use `user_facing_server_error`: there
/// the message content is never actionable.
pub fn action_error_message(e: &ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(msg) => msg.clone(),
        // `_` rather than exhaustive arms: `WrappedServerError` is
        // #[deprecated] in server_fn 0.8 and naming it fails -D warnings.
        _ => user_facing_server_error(e),
    }
}
