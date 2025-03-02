use dioxus::prelude::*;

use views::{Blog, Home, Login};

mod components;
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
    #[route("/login")]
    Login {}, 
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const STYLE_CSS: Asset = asset!("/assets/styling/style.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // Global app resources
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: STYLE_CSS }

        Router::<Route> {}
    }
}
