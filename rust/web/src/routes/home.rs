use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    rsx! {
        div { class: "container mx-auto px-4 py-8",
            div { class: "text-center",
                h1 { class: "text-4xl font-bold mb-4", "Welcome to Brdg.me" }
                p { class: "text-lg text-gray-600 mb-8", 
                    "Play board games online with friends"
                }
                div { class: "space-x-4",
                    Link { 
                        to: "/games",
                        class: "bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded",
                        "Browse Games"
                    }
                    Link { 
                        to: "/login",
                        class: "bg-green-500 hover:bg-green-700 text-white font-bold py-2 px-4 rounded",
                        "Login"
                    }
                }
            }
        }
    }
}