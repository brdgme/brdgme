use crate::game::server_fns::{
    BumpBotTurns, ConcedeGame, GameViewData, PlayerViewData, RestartGame, SubmitCommand, UndoGame,
};
use leptos::prelude::*;
use leptos_router::{NavigateOptions, hooks::use_navigate};
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
    let can_undo = data.can_undo;
    let is_finished = data.is_finished;
    let is_2player = data.is_2player;
    let restarted_game_id = data.restarted_game_id;
    let can_restart = is_finished && restarted_game_id.is_none();

    let has_bot_waiting = data.players.iter().any(|p| p.is_bot && p.is_turn);

    let trigger = expect_context::<crate::websocket_client::WebSocketTrigger>();
    let game_update = expect_context::<RwSignal<Option<(Uuid, u64)>>>();
    let undo_action = ServerAction::<UndoGame>::new();
    let concede_action = ServerAction::<ConcedeGame>::new();
    let restart_action = ServerAction::<RestartGame>::new();
    let bump_bot_action = ServerAction::<BumpBotTurns>::new();

    // Trigger re-fetch after undo/concede. Local bump makes the own action
    // refetch even if the WS is down; the trigger bump is still needed for
    // the layout header.
    Effect::new(move |_| {
        if let Some(Ok(())) = undo_action.value().get() {
            trigger.set_last_update.update(|n| *n += 1);
            crate::websocket_client::bump_game_update(game_update, game_id);
        }
    });
    Effect::new(move |_| {
        if let Some(Ok(())) = concede_action.value().get() {
            trigger.set_last_update.update(|n| *n += 1);
            crate::websocket_client::bump_game_update(game_update, game_id);
        }
    });

    // Navigate to new game after restart.
    let navigate = use_navigate();
    Effect::new(move |_| {
        if let Some(Ok(new_id)) = restart_action.value().get() {
            navigate(&format!("/games/{}", new_id), NavigateOptions::default());
        }
    });

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
                        <Show when=move || can_undo>
                            <div>
                                <a href="#" on:click=move |ev| {
                                    ev.prevent_default();
                                    undo_action.dispatch(UndoGame { game_id });
                                }>"Undo"</a>
                            </div>
                        </Show>
                        <Show when=move || !is_finished && is_2player>
                            <div>
                                <a href="#" on:click=move |ev| {
                                    ev.prevent_default();
                                    let confirmed = web_sys::window()
                                        .and_then(|w| w.confirm_with_message("Are you sure you want to concede?").ok())
                                        .unwrap_or(false);
                                    if confirmed {
                                        concede_action.dispatch(ConcedeGame { game_id });
                                    }
                                }>"Concede"</a>
                            </div>
                        </Show>
                        <Show when=move || can_restart>
                            <div>
                                <a href="#" on:click=move |ev| {
                                    ev.prevent_default();
                                    restart_action.dispatch(RestartGame { game_id });
                                }>"Restart"</a>
                            </div>
                        </Show>
                        <Show when=move || restarted_game_id.is_some()>
                            <div>
                                <a href=move || restarted_game_id.map(|id| format!("/games/{}", id)).unwrap_or_default()>
                                    "Go to new game"
                                </a>
                            </div>
                        </Show>
                        <Show when=move || has_bot_waiting>
                            <div>
                                <a href="#" on:click=move |ev| {
                                    ev.prevent_default();
                                    bump_bot_action.dispatch(BumpBotTurns { game_id });
                                }>"Bump bot to play"</a>
                            </div>
                        </Show>
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
                <PlayerName name=player.name color=player.color />
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
    format!(
        "{} {}, {}:{:02} {}",
        month_abbr,
        dt.day(),
        hour12,
        dt.minute(),
        ampm
    )
}

fn render_log_entries(
    entries: Vec<crate::game::server_fns::GameLogEntry>,
    show_timestamp: bool,
) -> impl IntoView {
    // Group into 10-minute windows; show log-time only on first entry of each group.
    let mut windows: Vec<(time::PrimitiveDateTime, Vec<_>)> = vec![];
    for entry in entries {
        let key = window_key(entry.logged_at);
        if let Some(last) = windows.last_mut()
            && last.0 == key
        {
            last.1.push(entry);
            continue;
        }
        windows.push((key, vec![entry]));
    }
    windows
        .into_iter()
        .map(move |(window_start, entries)| {
            let time_label = format_log_time(window_start);
            entries
                .into_iter()
                .enumerate()
                .map(move |(i, entry)| {
                    let label = if show_timestamp && i == 0 {
                        Some(format!("- {} -", time_label))
                    } else {
                        None
                    };
                    view! {
                        <div class="game-log-entry">
                            {label.map(|l| view! { <div class="log-time">{l}</div> })}
                            <div inner_html=entry.body_html />
                        </div>
                    }
                })
                .collect_view()
        })
        .collect_view()
}

#[component]
pub fn GameLogs(game_id: Uuid) -> impl IntoView {
    use crate::game::server_fns::get_game_logs;

    let game_update = expect_context::<RwSignal<Option<(Uuid, u64)>>>();
    let seq_for_this_game = Memo::new(move |_| {
        game_update
            .get()
            .and_then(|(id, seq)| (id == game_id).then_some(seq))
    });
    let logs = LocalResource::new(move || async move {
        let _ = seq_for_this_game.get();
        get_game_logs(game_id).await
    });

    let logs_ref = NodeRef::<leptos::html::Div>::new();

    Effect::new(move |_| {
        let _ = logs.get();
        leptos::prelude::request_animation_frame(move || {
            if let Some(el) = logs_ref.get_untracked()
                && let Some(parent) = el.parent_element()
            {
                parent.set_scroll_top(parent.scroll_height());
            }
        });
    });

    view! {
        {move || logs.get().map(|result| match result {
            Err(_) => view! { <div>"Failed to load logs."</div> }.into_any(),
            Ok(entries) => view! {
                <div class="game-logs" node_ref=logs_ref>
                    {render_log_entries(entries, true)}
                </div>
            }.into_any(),
        })}
    }
}

#[component]
pub fn RecentGameLogs(game_id: Uuid) -> impl IntoView {
    use crate::game::server_fns::get_game_logs;

    let game_update = expect_context::<RwSignal<Option<(Uuid, u64)>>>();
    let seq_for_this_game = Memo::new(move |_| {
        game_update
            .get()
            .and_then(|(id, seq)| (id == game_id).then_some(seq))
    });
    let logs = LocalResource::new(move || async move {
        let _ = seq_for_this_game.get();
        get_game_logs(game_id).await
    });

    let recent_ref = NodeRef::<leptos::html::Div>::new();

    Effect::new(move |_| {
        let _ = logs.get();
        leptos::prelude::request_animation_frame(move || {
            if let Some(el) = recent_ref.get_untracked() {
                el.set_scroll_top(el.scroll_height());
            }
        });
    });

    view! {
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
                            <div class="recent-logs game-logs" node_ref=recent_ref>
                                {render_log_entries(recent, false)}
                            </div>
                        </div>
                    })
                }
            },
        })}
    }
}

#[component]
pub fn PlayerName(name: String, color: String) -> impl IntoView {
    view! {
        <strong style=format!("color:{}", color)>"<" {name} ">"</strong>
    }
}

fn word_prefix(input: &str) -> &str {
    match input.rfind(' ') {
        Some(i) => &input[..i + 1],
        None => "",
    }
}

#[component]
pub fn GameCommandInput(
    game_id: uuid::Uuid,
    command_spec: Option<brdgme_game::command::Spec>,
    player_names: Vec<String>,
) -> impl IntoView {
    let (command, set_command) = signal(String::new());
    let trigger = expect_context::<crate::websocket_client::WebSocketTrigger>();
    let game_update = expect_context::<RwSignal<Option<(Uuid, u64)>>>();
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let submit_action = ServerAction::<SubmitCommand>::new();

    // Focus input on mount (works for both hard refresh and client-side navigation).
    Effect::new(move |_| {
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    });

    // Type-anywhere-focuses-command-field: only single, unmodified,
    // printable-character keystrokes are diverted, and only when nothing is
    // already focused - so Tab-focused links/buttons keep their normal
    // keyboard behaviour, especially Enter navigating a focused link.
    let keydown_listener = window_event_listener(leptos::ev::keydown, move |ev| {
        if ev.ctrl_key() || ev.meta_key() || ev.alt_key() {
            return;
        }
        if ev.key().chars().count() != 1 {
            return;
        }
        let nothing_focused = document()
            .active_element()
            .map(|el| el.tag_name() == "BODY")
            .unwrap_or(true);
        if !nothing_focused {
            return;
        }
        if let Some(el) = input_ref.get_untracked() {
            let _ = el.focus();
        }
    });
    on_cleanup(move || keydown_listener.remove());

    // Clear command, refocus input, and trigger re-fetch on successful submit.
    // Local bump makes the own action refetch even if the WS is down; the
    // trigger bump is still needed for the layout header.
    Effect::new(move |_| {
        if let Some(Ok(_)) = submit_action.value().get() {
            set_command.set(String::new());
            trigger.set_last_update.update(|n| *n += 1);
            crate::websocket_client::bump_game_update(game_update, game_id);
            if let Some(el) = input_ref.get() {
                let _ = el.focus();
            }
        }
    });

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let cmd = command.get_untracked();
        if !cmd.is_empty() {
            submit_action.dispatch(SubmitCommand {
                game_id,
                command: cmd,
            });
        }
    };

    let suggestions = Memo::new(move |_| -> Vec<brdgme_game::command::Suggestion> {
        let current_input = command.get();
        let Some(ref spec) = command_spec else {
            return vec![];
        };
        spec.suggest(&current_input, &player_names)
    });

    let error_msg = move || {
        submit_action.value().get().and_then(|r| match r {
            Err(e) => Some(e.to_string()),
            Ok(_) => None,
        })
    };

    view! {
        <>
            <Show when=move || !suggestions.get().is_empty()>
                <div class="suggestions-container">
                    <div class="suggestions-content">
                        {move || {
                            // Group consecutive suggestions sharing the same desc.
                            let mut groups: Vec<(Option<String>, Vec<String>)> = vec![];
                            for s in suggestions.get() {
                                if let Some(last) = groups.last_mut()
                                    && last.0 == s.desc {
                                        last.1.push(s.value);
                                        continue;
                                    }
                                groups.push((s.desc, vec![s.value]));
                            }
                            groups.into_iter().map(|(desc, values)| {
                                let make_link = |value: String| {
                                    let value2 = value.clone();
                                    let on_click = move |ev: leptos::ev::MouseEvent| {
                                        ev.prevent_default();
                                        let current = command.get_untracked();
                                        let prefix = word_prefix(&current);
                                        set_command.set(format!("{}{} ", prefix, value2));
                                    };
                                    view! { <a href="#" on:click=on_click>{value}</a> }
                                };
                                if values.len() == 1 {
                                    let value = values.into_iter().next().unwrap();
                                    leptos::either::Either::Left(view! {
                                        <div class="suggestion-doc-item">
                                            {make_link(value)}
                                            {desc.map(|d| view! {
                                                <span class="suggestion-doc-desc">" - " {d}</span>
                                            })}
                                        </div>
                                    })
                                } else {
                                    let value_views = values.into_iter()
                                        .map(|v| view! { <span>{make_link(v)}" "</span> })
                                        .collect_view();
                                    leptos::either::Either::Right(view! {
                                        <div class="suggestion-doc">
                                            {desc.map(|d| view! {
                                                <div class="suggestion-doc-header">
                                                    <span class="suggestion-doc-desc">{d}</span>
                                                </div>
                                            })}
                                            <div class="suggestion-doc-values">{value_views}</div>
                                        </div>
                                    })
                                }
                            }).collect_view()
                        }}
                    </div>
                </div>
            </Show>
            <div class="game-command-input">
                {move || error_msg().map(|e| view! {
                    <div class="command-error">{e}</div>
                })}
                <form on:submit=on_submit>
                    <input
                        type="text"
                        placeholder="Enter command..."
                        autocomplete="off"
                        autocapitalize="none"
                        spellcheck="false"
                        node_ref=input_ref
                        prop:value=command
                        on:input=move |ev| set_command.set(event_target_value(&ev))
                    />
                    <input type="submit" value="Send"/>
                </form>
            </div>
        </>
    }
}
