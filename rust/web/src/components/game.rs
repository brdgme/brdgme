use leptos::prelude::*;
use crate::game::server_fns::{GameViewData, PlayerViewData, SubmitCommand};
use brdgme_game::command::parser::Parser;
use uuid::Uuid;

#[component]
pub fn GameBoard(html: String) -> impl IntoView {
    view! {
        <div class="game-render"><pre inner_html=html></pre></div>
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

fn window_key(dt: time::PrimitiveDateTime) -> time::PrimitiveDateTime {
    let minute = dt.minute() / 10 * 10;
    time::PrimitiveDateTime::new(
        dt.date(),
        time::Time::from_hms(dt.hour(), minute, 0).unwrap_or(dt.time()),
    )
}

fn format_window(dt: time::PrimitiveDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        dt.year(), dt.month() as u8, dt.day(), dt.hour(), dt.minute()
    )
}

#[component]
pub fn GameLogs(game_id: Uuid) -> impl IntoView {
    use crate::game::server_fns::get_game_logs;

    let trigger = expect_context::<crate::websocket_client::WebSocketTrigger>();
    let logs = Resource::new(
        move || (game_id, trigger.last_update.get()),
        |(id, _)| async move { get_game_logs(id).await },
    );

    view! {
        <div class="recent-logs-container">
            <Suspense fallback=|| ()>
                {move || logs.get().map(|result| match result {
                    Err(_) => view! { <div class="recent-logs-error">"Failed to load logs."</div> }.into_any(),
                    Ok(entries) => {
                        // Group into 10-minute windows
                        let mut windows: Vec<(time::PrimitiveDateTime, Vec<_>)> = vec![];
                        for entry in entries {
                            let key = window_key(entry.logged_at);
                            if let Some(last) = windows.last_mut() {
                                if last.0 == key {
                                    last.1.push(entry);
                                    continue;
                                }
                            }
                            windows.push((key, vec![entry]));
                        }
                        view! {
                            <div class="recent-logs">
                                {windows.into_iter().map(|(window_start, entries)| {
                                    let heading = format_window(window_start);
                                    view! {
                                        <div class="log-window">
                                            <div class="log-window-heading">{heading}</div>
                                            {entries.into_iter().map(|entry| view! {
                                                <div
                                                    class="log-entry"
                                                    class:log-entry-new=entry.is_new
                                                    inner_html=entry.body_html
                                                />
                                            }).collect_view()}
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    },
                })}
            </Suspense>
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