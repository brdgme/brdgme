use crate::game::server_fns::{
    BumpBotTurns, ConcedeGame, ForceDeleteGame, GameViewData, PlayerViewData, RestartGame,
    SubmitCommand, UndoGame,
};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::{NavigateOptions, hooks::use_navigate};
use uuid::Uuid;
use web_sys::wasm_bindgen::JsCast;

/// The game command input's text, owned by `GamePage` (above the
/// `<Transition>` closure that remounts `GameCommandInput` on every game
/// refetch) so typed input survives background updates. Newtype so the
/// context can't collide with other RwSignal<String> providers.
#[derive(Clone, Copy)]
pub struct CommandInputText(pub RwSignal<String>);

#[component]
pub fn GameBoard(html: String, player_style: String) -> impl IntoView {
    view! {
        <div class="game-render" style=player_style><pre inner_html=html></pre></div>
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
    let viewer_is_admin = data.viewer_is_admin;
    let player_style = data.player_style.clone();

    let trigger = expect_context::<crate::websocket_client::WebSocketTrigger>();
    let game_update = expect_context::<RwSignal<Option<(Uuid, u64)>>>();
    let undo_action = ServerAction::<UndoGame>::new();
    let concede_action = ServerAction::<ConcedeGame>::new();
    let restart_action = ServerAction::<RestartGame>::new();
    let bump_bot_action = ServerAction::<BumpBotTurns>::new();
    let force_delete_action = ServerAction::<ForceDeleteGame>::new();

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

    // Navigate away after force delete (spec D3); bump the sidebar trigger so
    // the deleted game drops out of the active-games list.
    let navigate_after_delete = use_navigate();
    Effect::new(move |_| {
        if let Some(Ok(())) = force_delete_action.value().get() {
            trigger.set_last_update.update(|n| *n += 1);
            navigate_after_delete("/", NavigateOptions::default());
        }
    });

    // Header "Sub menu" button state, provided by `MainLayout`. Drives the
    // narrow-viewport overlay (see the `@media (max-width: 60em)` block in
    // main.scss); underlay mounted only while open, like the sidebar menu.
    let sub_menu = expect_context::<crate::components::layout::SubMenuOpen>();

    view! {
        <Show when=move || sub_menu.open.get()>
            <div
                class="game-meta-close-underlay"
                on:click=move |_| sub_menu.set_open.set(false)
            ></div>
        </Show>
        <div class="game-meta" class:open=move || sub_menu.open.get()>
            <div class="game-meta-main">
                <div>
                    <h2>{data.type_name}</h2>
                    {
                        let viewer_user_id = data.viewer_user_id;
                        data.players.into_iter().map(|p| view! {
                            <PlayerInfo player=p viewer_user_id=viewer_user_id />
                        }).collect_view()
                    }
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
                        <Show when=move || has_bot_waiting && viewer_is_admin>
                            <div>
                                <a href="#" on:click=move |ev| {
                                    ev.prevent_default();
                                    bump_bot_action.dispatch(BumpBotTurns { game_id });
                                }>"Bump bot to play"</a>
                            </div>
                        </Show>
                        <Show when=move || viewer_is_admin>
                            <div>
                                <a href=format!("/admin/games/{}/export", game_id)>
                                    "Export JSON (admin)"
                                </a>
                            </div>
                            <div>
                                <a href="#" on:click=move |ev| {
                                    ev.prevent_default();
                                    let confirmed = web_sys::window()
                                        .and_then(|w| w.confirm_with_message("Permanently delete this game for all players? This cannot be undone.").ok())
                                        .unwrap_or(false);
                                    if confirmed {
                                        force_delete_action.dispatch(ForceDeleteGame { game_id });
                                    }
                                }>"Delete game (admin)"</a>
                            </div>
                        </Show>
                    </div>
                </div>
            </div>
            <div class="game-meta-logs">
                <h2>"Logs"</h2>
                <div class="game-meta-logs-content">
                    // logs is a LocalResource that never resolves on SSR: this Suspense keeps
                    // the outer Transition from emitting fallback HTML on the server, and the
                    // mounted-gate inside GameLogs keeps SSR and hydration output identical
                    // (see the comment in GameLogs).
                    <Suspense fallback=|| ()>
                        <GameLogs player_style=player_style />
                    </Suspense>
                </div>
            </div>
        </div>
    }
}

#[component]
fn PlayerInfo(player: PlayerViewData, viewer_user_id: Option<Uuid>) -> impl IntoView {
    // Legacy presentation: "Rating: 1216 (<icon>16)" - icon U+2197 (up,
    // green) / U+2198 (down, red) / "-" (zero, blue), number always the
    // absolute value. Only rendered once a rating change exists.
    let rating_change = player.rating_change.map(|amount| {
        let (class, icon) = match amount {
            a if a > 0 => ("rating-change-up", "\u{2197}"),
            a if a < 0 => ("rating-change-down", "\u{2198}"),
            _ => ("rating-change-none", "-"),
        };
        view! {
            <span>
                " ("
                <span class="rating-change">
                    <span class=class>{icon}</span>
                    {amount.abs()}
                </span>
                ")"
            </span>
        }
    });
    let form = player.form;
    view! {
        <div class="player-info">
            <div class:brdgme-is-turn=player.is_turn>
                <PlayerName
                    name=player.name
                    color=player.color
                    profile_link=!player.is_bot
                />
                {(!form.is_empty()).then(|| view! {
                    <span class="player-form">" "<crate::stats::viz::FormStrip results=form/></span>
                })}
            </div>
            <div style="margin-left: 1em;">
                <div>
                    <abbr title="ELO rating" style="cursor: help;">"Rating"</abbr>
                    ": " {player.rating} {rating_change}
                </div>
                <div>"Points: " {player.points}</div>
                {player.user_id
                    .filter(|uid| viewer_user_id.is_some() && Some(*uid) != viewer_user_id)
                    .map(|uid| {
                        let add_friend = ServerAction::<crate::friends::SendFriendRequest>::new();
                        view! {
                            <div>
                                {move || match add_friend.value().get() {
                                    Some(Ok(())) => view! { <span>"Friend request sent"</span> }.into_any(),
                                    Some(Err(e)) => view! { <span class="error">{e.to_string()}</span> }.into_any(),
                                    None => view! {
                                        <a href="#" on:click=move |ev| {
                                            ev.prevent_default();
                                            add_friend.dispatch(crate::friends::SendFriendRequest {
                                                user_id: Some(uid),
                                                name: None,
                                            });
                                        }>"Add friend"</a>
                                    }.into_any(),
                                }}
                            </div>
                        }
                    })}
            </div>
        </div>
    }
}

// 10-minute buckets on the UTC timeline; instant-based, so DST-immune.
fn window_key(dt: time::PrimitiveDateTime) -> i64 {
    dt.assume_utc().unix_timestamp() / 600
}

// Formats in the browser's local time zone via Date.toLocaleString, e.g. "Jul 11, 10:50 AM".
// Only runs client-side (render_log_entries is reached via LocalResource).
fn format_log_time(window: i64) -> String {
    let date = js_sys::Date::new(&((window * 600_000) as f64).into());
    let options = js_sys::Object::new();
    for (key, value) in [
        ("month", "short"),
        ("day", "numeric"),
        ("hour", "numeric"),
        ("minute", "2-digit"),
    ] {
        let _ = js_sys::Reflect::set(&options, &key.into(), &value.into());
    }
    let _ = js_sys::Reflect::set(&options, &"hour12".into(), &true.into());
    date.to_locale_string("en-US", &options.into()).into()
}

fn render_log_entries(
    entries: Vec<crate::game::server_fns::GameLogEntry>,
    show_timestamp: bool,
) -> impl IntoView {
    // Group into 10-minute windows; show log-time only on first entry of each group.
    let mut windows: Vec<(i64, Vec<_>)> = vec![];
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
pub fn GameLogs(player_style: String) -> impl IntoView {
    // Provided by GamePage in app.rs, above the <Transition> closure that
    // remounts this component on every command submit - see the comment at
    // its `provide_context` call site there. Reading it via context instead
    // of creating a local LocalResource keeps the last logs visible across
    // remounts instead of flashing blank.
    let logs = expect_context::<
        LocalResource<Result<Vec<crate::game::server_fns::GameLogEntry>, ServerFnError>>,
    >();

    // Effects never run during SSR, so `mounted` is false in the server
    // HTML and on the client's initial hydration pass - both sides render
    // nothing, sidestepping a hydration-cursor mismatch (the Suspense
    // boundaries around this component get different SerializedDataIds on
    // the server vs the client, so the client hydrates against fallback
    // HTML). Logs appear right after mount; the shared LocalResource has
    // been fetching since GamePage created it, so there's no added delay.
    let mounted = RwSignal::new(false);
    Effect::new(move |_| mounted.set(true));

    let logs_ref = NodeRef::<leptos::html::Div>::new();

    Effect::new(move |_| {
        let _ = logs.get();
        leptos::prelude::request_animation_frame(move || {
            // try_: the raf can fire after this scope is disposed
            // (navigation/unmount); get_untracked would panic then.
            if let Some(el) = logs_ref.try_get_untracked().flatten()
                && let Some(parent) = el.parent_element()
            {
                parent.set_scroll_top(parent.scroll_height());
            }
        });
    });

    view! {
        {move || mounted.get().then(|| logs.get()).flatten().map(|result| match result {
            Err(_) => view! { <div>"Failed to load logs."</div> }.into_any(),
            Ok(entries) => view! {
                <div class="game-logs" node_ref=logs_ref style=player_style.clone()>
                    {render_log_entries(entries, true)}
                </div>
            }.into_any(),
        })}
    }
}

#[component]
pub fn RecentGameLogs(player_style: String) -> impl IntoView {
    // See the comment in `GameLogs` above - this reads the same
    // GamePage-hoisted resource so it doesn't flash blank on remount.
    let logs = expect_context::<
        LocalResource<Result<Vec<crate::game::server_fns::GameLogEntry>, ServerFnError>>,
    >();

    // See the mounted-gate comment in GameLogs above.
    let mounted = RwSignal::new(false);
    Effect::new(move |_| mounted.set(true));

    let recent_ref = NodeRef::<leptos::html::Div>::new();

    Effect::new(move |_| {
        let _ = logs.get();
        leptos::prelude::request_animation_frame(move || {
            // try_: see the comment in GameLogs above.
            if let Some(el) = recent_ref.try_get_untracked().flatten() {
                el.set_scroll_top(el.scroll_height());
            }
        });
    });

    view! {
        {move || mounted.get().then(|| logs.get()).flatten().map(|result| match result {
            Err(_) => None,
            Ok(entries) => {
                let recent: Vec<_> = entries.into_iter().filter(|e| e.is_new).collect();
                if recent.is_empty() {
                    None
                } else {
                    Some(view! {
                        <div class="recent-logs-container" style=player_style.clone()>
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

/// `color` is a `--mk-{slot}` colour slot token (e.g. "green"), not a hex
/// value - display always follows the active theme.
#[component]
pub fn PlayerName(
    name: String,
    color: String,
    #[prop(optional)] profile_link: bool,
) -> impl IntoView {
    let strong = view! {
        <strong style=format!("color:var(--mk-{})", color)>"<" {name.clone()} ">"</strong>
    };
    if profile_link {
        view! {
            <A href=format!("/players/{}", crate::players::encode_path_segment(&name))>
                {strong}
            </A>
        }
        .into_any()
    } else {
        strong.into_any()
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
    let command = expect_context::<CommandInputText>().0;
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

    // Type-anywhere-focuses-command-field: single, unmodified,
    // printable-character keystrokes focus the command input unless a
    // text-entry element (input/textarea/select/contenteditable) is
    // focused. Enter and Tab are already excluded (not single chars), and
    // Space is left alone whenever anything other than the body is
    // focused, so Tab-focused links/buttons keep their normal keyboard
    // behaviour (Space activating them, Enter navigating a focused link).
    let keydown_listener = window_event_listener(leptos::ev::keydown, move |ev| {
        if ev.ctrl_key() || ev.meta_key() || ev.alt_key() {
            return;
        }
        if ev.key().chars().count() != 1 {
            return;
        }
        let active_element = document().active_element();
        let is_text_entry = active_element
            .as_ref()
            .map(|el| {
                matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                    || el
                        .dyn_ref::<web_sys::HtmlElement>()
                        .map(|html_el| html_el.is_content_editable())
                        .unwrap_or(false)
            })
            .unwrap_or(false);
        if is_text_entry {
            return;
        }
        let is_body = active_element
            .map(|el| el.tag_name() == "BODY")
            .unwrap_or(true);
        if ev.key() == " " && !is_body {
            return;
        }
        if let Some(el) = input_ref.get_untracked() {
            let _ = el.focus();
        }
    });
    on_cleanup(move || keydown_listener.remove());

    // On any submit outcome, refocus the input (disabling it while pending
    // drops focus). On success additionally clear the text and trigger a
    // re-fetch - the local bump makes the own action refetch even if the WS
    // is down; the trigger bump is still needed for the layout header. On
    // error the text signal is left alone so the user can correct it.
    Effect::new(move |_| {
        let Some(result) = submit_action.value().get() else {
            return;
        };
        if matches!(result, Ok(None)) {
            command.set(String::new());
            trigger.set_last_update.update(|n| *n += 1);
            crate::websocket_client::bump_game_update(game_update, game_id);
        }
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
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
            // Game-rejected command: expected user-input feedback.
            Ok(Some(msg)) => Some(format!("Invalid command: {}", msg)),
            Ok(None) => None,
            // Transport/server fault: never leak the raw ServerFnError text.
            Err(_) => Some("Failed to submit command. Please try again.".to_string()),
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
                                        command.set(format!("{}{} ", prefix, value2));
                                        // The click focuses the <a>, so the
                                        // type-anywhere guard won't help here -
                                        // refocus the input directly.
                                        if let Some(el) = input_ref.get_untracked() {
                                            let _ = el.focus();
                                        }
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
                        disabled=move || submit_action.pending().get()
                        on:input=move |ev| command.set(event_target_value(&ev))
                    />
                    <input
                        type="submit"
                        value="Send"
                        disabled=move || submit_action.pending().get()
                    />
                </form>
            </div>
        </>
    }
}
