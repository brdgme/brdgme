use leptos::prelude::*;

/// The reusable form-row template (see the 2026-07-16 settings spec):
/// bold block label above the control, optional muted help line, optional
/// red error line. CSS lives in main.scss under `.form-*`.
#[component]
pub fn FormField(
    #[prop(into)] label: String,
    #[prop(optional, into)] help: Option<String>,
    #[prop(optional, into)] error: Signal<Option<String>>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="form-field">
            <label class="form-label">{label}</label>
            <div class="form-control">{children()}</div>
            {help.map(|h| view! { <div class="form-help">{h}</div> })}
            {move || error.get().map(|e| view! { <div class="form-error">{e}</div> })}
        </div>
    }
}

/// A colour swatch: the canonical colour name (e.g. "Green") padded with one
/// space either side, slot colour as background, contrast colour as text -
/// reuses the `mk-bg-*`/`mk-fg-c-*` markup classes so it previews in the
/// live theme.
#[component]
pub fn ColorChip(#[prop(into)] color: Signal<String>) -> impl IntoView {
    view! {
        <span class=move || {
            let slot = crate::theme::slot_from_color_name(&color.get());
            format!("color-chip mk-bg-{slot} mk-fg-c-{slot}")
        }>
            {move || format!(" {} ", color.get())}
        </span>
    }
}
