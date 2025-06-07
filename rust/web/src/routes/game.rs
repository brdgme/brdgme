use dioxus::prelude::*;
use crate::models::{Game as GameModel, GamePlayer, GameLog};

#[component]
pub fn Game(id: String) -> Element {
    let mut game = use_signal(|| None::<GameModel>);
    let mut players = use_signal(|| Vec::<GamePlayer>::new());
    let mut logs = use_signal(|| Vec::<GameLog>::new());
    let mut loading = use_signal(|| true);
    let mut command_input = use_signal(|| String::new());
    let mut sending_command = use_signal(|| false);

    // Load game data on component mount
    use_effect(move || {
        spawn(async move {
            loading.set(true);
            
            // TODO: Implement get_game, get_game_players, get_game_logs server functions
            // For now, simulate loading
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            
            // Mock data for now
            game.set(None);
            players.set(vec![]);
            logs.set(vec![]);
            loading.set(false);
        });
    });

    let handle_command_submit = move |_| {
        if command_input().trim().is_empty() {
            return;
        }
        
        spawn(async move {
            sending_command.set(true);
            
            // TODO: Implement submit_command server function
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            
            command_input.set(String::new());
            sending_command.set(false);
        });
    };

    rsx! {
        div { class: "container mx-auto px-4 py-8",
            if loading() {
                div { class: "flex justify-center items-center py-12",
                    div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" }
                }
            } else if let Some(game_data) = game() {
                div { class: "grid lg:grid-cols-3 gap-8",
                    // Main game area
                    div { class: "lg:col-span-2 space-y-6",
                        // Game header
                        div { class: "bg-white rounded-lg shadow-md p-6",
                            div { class: "flex justify-between items-start mb-4",
                                h1 { class: "text-2xl font-bold", "Game {id}" }
                                span { 
                                    class: if game_data.is_finished { "bg-gray-100 text-gray-800" } else { "bg-green-100 text-green-800" },
                                    class: "px-3 py-1 rounded-full text-sm font-medium",
                                    if game_data.is_finished { "Finished" } else { "Active" }
                                }
                            }
                            p { class: "text-gray-600",
                                "Created: {game_data.created_at}"
                            }
                        }

                        // Game board/state area
                        div { class: "bg-white rounded-lg shadow-md p-6",
                            h2 { class: "text-xl font-semibold mb-4", "Game Board" }
                            div { class: "bg-gray-100 rounded-lg p-8 text-center",
                                p { class: "text-gray-600", "Game rendering will go here" }
                                p { class: "text-sm text-gray-500 mt-2", "This will display the game state visually" }
                            }
                        }

                        // Command input (if game is active)
                        if !game_data.is_finished {
                            div { class: "bg-white rounded-lg shadow-md p-6",
                                h3 { class: "text-lg font-semibold mb-4", "Enter Command" }
                                form { 
                                    onsubmit: move |evt| {
                                        evt.prevent_default();
                                        handle_command_submit(evt);
                                    },
                                    div { class: "flex space-x-2",
                                        input {
                                            r#type: "text",
                                            placeholder: "Enter your command...",
                                            class: "flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                                            value: command_input(),
                                            oninput: move |evt| command_input.set(evt.value()),
                                            disabled: sending_command()
                                        }
                                        button {
                                            r#type: "submit",
                                            disabled: sending_command() || command_input().trim().is_empty(),
                                            class: "px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed",
                                            if sending_command() {
                                                "Sending..."
                                            } else {
                                                "Submit"
                                            }
                                        }
                                    }
                                }
                                div { class: "mt-2 text-sm text-gray-600",
                                    "Available commands will be shown here with autocomplete"
                                }
                            }
                        }
                    }

                    // Sidebar
                    div { class: "space-y-6",
                        // Players panel
                        div { class: "bg-white rounded-lg shadow-md p-6",
                            h3 { class: "text-lg font-semibold mb-4", "Players" }
                            div { class: "space-y-3",
                                if players().is_empty() {
                                    p { class: "text-gray-500 text-sm", "No players loaded" }
                                } else {
                                    for player in players() {
                                        PlayerCard { player: player }
                                    }
                                }
                            }
                        }

                        // Game actions
                        div { class: "bg-white rounded-lg shadow-md p-6",
                            h3 { class: "text-lg font-semibold mb-4", "Actions" }
                            div { class: "space-y-2",
                                button {
                                    class: "w-full px-4 py-2 bg-gray-500 text-white rounded-md hover:bg-gray-600",
                                    "Game History"
                                }
                                if !game_data.is_finished {
                                    button {
                                        class: "w-full px-4 py-2 bg-yellow-500 text-white rounded-md hover:bg-yellow-600",
                                        "Undo Last Move"
                                    }
                                    button {
                                        class: "w-full px-4 py-2 bg-red-500 text-white rounded-md hover:bg-red-600",
                                        "Resign"
                                    }
                                }
                            }
                        }

                        // Game logs panel
                        div { class: "bg-white rounded-lg shadow-md p-6",
                            h3 { class: "text-lg font-semibold mb-4", "Game Log" }
                            div { class: "max-h-96 overflow-y-auto space-y-2",
                                if logs().is_empty() {
                                    p { class: "text-gray-500 text-sm", "No logs yet" }
                                } else {
                                    for log in logs() {
                                        LogEntry { log: log }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "text-center py-12",
                    h2 { class: "text-2xl font-bold text-gray-900 mb-4", "Game Not Found" }
                    p { class: "text-gray-600 mb-6", "The game you're looking for doesn't exist or you don't have access to it." }
                    Link {
                        to: "/games",
                        class: "bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded",
                        "Back to Games"
                    }
                }
            }
        }
    }
}

#[component]
fn PlayerCard(player: GamePlayer) -> Element {
    rsx! {
        div { class: "flex items-center space-x-3 p-3 rounded-lg",
            class: if player.is_turn { "bg-blue-50 border-2 border-blue-200" } else { "bg-gray-50" },
            div { 
                class: "w-10 h-10 rounded-full flex items-center justify-center text-white font-bold",
                style: "background-color: {player.color}",
                "{player.position}"
            }
            div { class: "flex-1",
                p { class: "font-medium", "Player {player.position}" }
                if player.is_turn {
                    p { class: "text-sm text-blue-600", "Current turn" }
                } else if player.is_eliminated {
                    p { class: "text-sm text-red-600", "Eliminated" }
                } else {
                    p { class: "text-sm text-gray-600", "Waiting" }
                }
                if let Some(points) = player.points {
                    p { class: "text-xs text-gray-500", "Points: {points}" }
                }
            }
            if !player.is_read {
                div { class: "w-2 h-2 bg-red-500 rounded-full" }
            }
        }
    }
}

#[component]
fn LogEntry(log: GameLog) -> Element {
    rsx! {
        div { class: "text-sm border-l-2 border-gray-200 pl-3 py-1",
            div { class: "flex justify-between items-start",
                p { class: "text-gray-800", "{log.body}" }
                span { class: "text-xs text-gray-500 ml-2", 
                    "{log.logged_at}"
                }
            }
        }
    }
}