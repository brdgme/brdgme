use leptos::prelude::*;
use crate::game::server_fns::{GameViewData, PlayerViewData, SubmitCommand};
use brdgme_game::command::parser::Parser;

#[component]
pub fn GameBoard(html: String) -> impl IntoView {
    view! {
        <div class="game-render" inner_html=html></div>
    }
}

#[component]
pub fn GameMeta(data: GameViewData) -> impl IntoView {
    view! {
        <div class="game-meta">
            <div class="game-meta-main">
                <div>
                    <h2>{data.type_name}</h2>
                    {data.players.into_iter().map(|p| view! {
                        <PlayerInfo player=p />
                    }).collect_view()}
                    <div class="game-actions">
                        <h3>"Actions"</h3>
                        <div><a>"Concede"</a></div>
                    </div>
                </div>
            </div>
            <div class="game-meta-logs">
                <h2>"Logs"</h2>
                <div class="game-meta-logs-content">
                    <div>"Full game logs here"</div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn PlayerInfo(player: PlayerViewData) -> impl IntoView {
    view! {
        <div class="player-info">
            <div class:brdgme-is-turn=player.is_turn>
                <strong>"<" {player.name} ">"</strong>
            </div>
            <div style="margin-left: 1em;">
                <div><abbr title="ELO rating" style="cursor: help;">"Rating"</abbr>": " {player.rating}</div>
                <div>"Points: " {player.points}</div>
            </div>
        </div>
    }
}

#[component]
pub fn GameLogs() -> impl IntoView {
    view! {
        <div class="recent-logs-container">
            <div class="recent-logs-header">"Recent logs"</div>
            <div class="recent-logs">
                <div>"Log entries would go here"</div>
            </div>
        </div>
    }
}

#[component]
pub fn GameCommandInput(
    game_id: uuid::Uuid, 
    command_spec: Option<brdgme_game::command::Spec>,
    player_names: Vec<String>,
) -> impl IntoView {
    let (command, set_command) = signal(String::new());
    
    let submit_action = ServerAction::<SubmitCommand>::new();
    
    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let cmd = command.get();
        if !cmd.is_empty() {
            submit_action.dispatch(SubmitCommand {
                game_id,
                command: cmd,
            });
            set_command.set(String::new());
        }
    };

    let suggestions = move || {
        let current_input = command.get();
        if let Some(ref spec) = command_spec {
            // Very basic suggestion logic for now: 
            // try to parse and get expected tokens
            match spec.parse(&current_input, &player_names) {
                Ok(out) => {
                    if out.remaining.is_empty() {
                        vec!["<enter>".to_string()]
                    } else {
                        // This case shouldn't happen much with recursive descent
                        vec![]
                    }
                }
                Err(brdgme_game::errors::GameError::Parse { expected, .. }) => {
                    expected
                }
                _ => vec![]
            }
        } else {
            vec![]
        }
    };

    view! {
        <div class="game-command-input-container">
            <div class="suggestions-container">
                <div class="suggestions-content">
                    {move || suggestions().into_iter().map(|s| view! {
                        <div class="suggestion-item">{s}</div>
                    }).collect_view()}
                </div>
            </div>
            <div class="game-command-input">
                <form on:submit=on_submit>
                    <input 
                        type="text" 
                        placeholder="Enter command..." 
                        autocomplete="off" 
                        autocapitalize="none" 
                        spellcheck="false"
                        prop:value=command
                        on:input=move |ev| set_command.set(event_target_value(&ev))
                    />
                    <input type="submit" value="Send"/>
                </form>
            </div>
        </div>
    }
}