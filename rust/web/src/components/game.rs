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
    let game_id = data.id;
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
                    <GameLogs game_id=game_id />
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

fn format_log_time(dt: time::PrimitiveDateTime) -> String {
    let month_abbr = match dt.month() {
        time::Month::January => "Jan",
        time::Month::February => "Feb",
        time::Month::March => "Mar",
        time::Month::April => "Apr",
        time::Month::May => "May",
        time::Month::June => "Jun",
        time::Month::July => "Jul",
        time::Month::August => "Aug",
        time::Month::September => "Sep",
        time::Month::October => "Oct",
        time::Month::November => "Nov",
        time::Month::December => "Dec",
    };
    let hour12 = dt.hour() % 12;
    let hour12 = if hour12 == 0 { 12 } else { hour12 };
    let ampm = if dt.hour() < 12 { "AM" } else { "PM" };
    format!("{} {}, {}:{:02} {}", month_abbr, dt.day(), hour12, dt.minute(), ampm)
}

fn render_log_entries(entries: Vec<crate::game::server_fns::GameLogEntry>) -> impl IntoView {
    // Group into 10-minute windows; show log-time only on first entry of each group.
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
    windows.into_iter().map(|(window_start, entries)| {
        let time_label = format_log_time(window_start);
        entries.into_iter().enumerate().map(move |(i, entry)| {
            let label = if i == 0 { format!("- {} -", time_label) } else { String::new() };
            view! {
                <div class="game-log-entry">
                    <div class="log-time">{label}</div>
                    <div inner_html=entry.body_html />
                </div>
            }
        }).collect_view()
    }).collect_view()
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
        <Suspense fallback=|| ()>
            {move || logs.get().map(|result| match result {
                Err(_) => view! { <div>"Failed to load logs."</div> }.into_any(),
                Ok(entries) => view! {
                    <div class="game-logs">
                        {render_log_entries(entries)}
                    </div>
                }.into_any(),
            })}
        </Suspense>
    }
}

#[component]
pub fn RecentGameLogs(game_id: Uuid) -> impl IntoView {
    use crate::game::server_fns::get_game_logs;

    let trigger = expect_context::<crate::websocket_client::WebSocketTrigger>();
    let logs = Resource::new(
        move || (game_id, trigger.last_update.get()),
        |(id, _)| async move { get_game_logs(id).await },
    );

    view! {
        <Suspense fallback=|| ()>
            {move || logs.get().map(|result| match result {
                Err(_) => None,
                Ok(entries) => {
                    let recent: Vec<_> = entries.into_iter().filter(|e| e.is_new).collect();
                    if recent.is_empty() {
                        None
                    } else {
                        Some(view! {
                            <div class="recent-logs-container">
                                <div class="recent-logs-header">"Recent logs"</div>
                                <div class="recent-logs game-logs">
                                    {render_log_entries(recent)}
                                </div>
                            </div>
                        })
                    }
                },
            })}
        </Suspense>
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