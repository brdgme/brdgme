use leptos::prelude::*;

/// The three-dot bounce spinner (styles: `.spinner` in main.scss). Markup
/// extracted from the login form so any page loading server data can reuse it.
#[component]
pub fn Spinner() -> impl IntoView {
    view! {
        <div class="spinner">
            <div class="bounce1"></div>
            <div class="bounce2"></div>
            <div class="bounce3"></div>
        </div>
    }
}
