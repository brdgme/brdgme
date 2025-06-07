use dioxus::prelude::*;

mod routes;

use routes::*;

#[derive(Debug, Clone, Routable, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/login")]
    Login {},
    #[route("/games")]
    Games {},
    #[route("/games/:id")]
    Game { id: String },
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: "https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css" }
        
        div { class: "min-h-screen bg-gray-50",
            // Navigation header
            nav { class: "bg-white shadow-sm border-b border-gray-200",
                div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                    div { class: "flex justify-between h-16",
                        div { class: "flex items-center",
                            Link { 
                                to: "/",
                                class: "text-xl font-bold text-gray-900",
                                "Brdg.me"
                            }
                        }
                        div { class: "flex items-center space-x-4",
                            Link { 
                                to: "/games",
                                class: "text-gray-700 hover:text-gray-900",
                                "Games"
                            }
                            Link { 
                                to: "/login",
                                class: "bg-blue-500 hover:bg-blue-700 text-white px-4 py-2 rounded",
                                "Login"
                            }
                        }
                    }
                }
            }
            
            // Main content
            main {
                Router::<Route> {}
            }
        }
    }
}