use dioxus::prelude::*;
use crate::models::*;

#[component]
pub fn Games() -> Element {
    let mut games = use_signal(|| Vec::<Game>::new());
    let mut loading = use_signal(|| true);

    // Load games on component mount
    use_effect(move || {
        spawn(async move {
            loading.set(true);
            
            // TODO: Implement get_games server function
            // For now, simulate loading
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            
            // Mock data for now
            games.set(vec![]);
            loading.set(false);
        });
    });

    rsx! {
        div { class: "container mx-auto px-4 py-8",
            div { class: "flex justify-between items-center mb-8",
                h1 { class: "text-3xl font-bold", "Your Games" }
                button {
                    class: "bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded",
                    "New Game"
                }
            }

            if loading() {
                div { class: "flex justify-center items-center py-12",
                    div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" }
                }
            } else if games().is_empty() {
                div { class: "text-center py-12",
                    div { class: "text-gray-500 text-lg mb-4",
                        "You don't have any active games"
                    }
                    button {
                        class: "bg-green-500 hover:bg-green-700 text-white font-bold py-2 px-4 rounded",
                        "Start Your First Game"
                    }
                }
            } else {
                div { class: "grid gap-6 md:grid-cols-2 lg:grid-cols-3",
                    for game in games() {
                        GameCard { game: game }
                    }
                }
            }

            div { class: "mt-12",
                h2 { class: "text-2xl font-bold mb-6", "Browse Game Types" }
                div { class: "grid gap-4 md:grid-cols-2 lg:grid-cols-4",
                    GameTypeCard { name: "Age of War", players: "2-8 players", weight: 1.5 }
                    GameTypeCard { name: "Battleship", players: "2 players", weight: 1.0 }
                    // Add more game types as needed
                }
            }
        }
    }
}

#[component]
fn GameCard(game: Game) -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow",
            div { class: "flex justify-between items-start mb-4",
                h3 { class: "text-xl font-semibold", "Game {game.id}" }
                span { 
                    class: if game.is_finished { "bg-gray-100 text-gray-800" } else { "bg-green-100 text-green-800" },
                    class: "px-2 py-1 rounded-full text-xs font-medium",
                    if game.is_finished { "Finished" } else { "Active" }
                }
            }
            
            div { class: "space-y-2 mb-4",
                p { class: "text-sm text-gray-600",
                    "Created: {game.created_at}"
                }
                if let Some(finished_at) = game.finished_at {
                    p { class: "text-sm text-gray-600",
                        "Finished: {finished_at}"
                    }
                }
            }
            
            div { class: "flex justify-between items-center",
                div { class: "flex space-x-2",
                    // Player avatars would go here
                    div { class: "w-8 h-8 bg-blue-500 rounded-full flex items-center justify-center text-white text-xs", "P1" }
                    div { class: "w-8 h-8 bg-red-500 rounded-full flex items-center justify-center text-white text-xs", "P2" }
                }
                Link {
                    to: "/games/{game.id}",
                    class: "bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded text-sm",
                    if game.is_finished { "View" } else { "Play" }
                }
            }
        }
    }
}

#[component]
fn GameTypeCard(name: String, players: String, weight: f32) -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-4 hover:shadow-lg transition-shadow cursor-pointer",
            h4 { class: "font-semibold text-lg mb-2", "{name}" }
            p { class: "text-sm text-gray-600 mb-2", "{players}" }
            div { class: "flex items-center",
                span { class: "text-xs text-gray-500", "Weight: " }
                div { class: "flex ml-1",
                    for _i in 0..(weight as usize) {
                        div { class: "w-2 h-2 bg-yellow-400 rounded-full mr-1" }
                    }
                    if weight.fract() > 0.0 {
                        div { class: "w-2 h-2 bg-yellow-200 rounded-full" }
                    }
                }
            }
        }
    }
}