use leptos::html;
use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    NavigateOptions, ParamSegment, StaticSegment,
    components::{A, Route, Router, Routes},
    hooks::use_navigate,
};
use uuid::Uuid;

use crate::auth::server::{confirm_login, login};
use crate::components::MainLayout;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="apple-mobile-web-app-capable" content="yes"/>
                <meta name="mobile-web-app-capable" content="yes"/>
                <link rel="icon" type="image/svg+xml" href="/favicon.svg"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let (last_update, set_last_update) = signal(0u64);
    provide_context(crate::websocket_client::WebSocketTrigger {
        last_update,
        set_last_update,
    });
    provide_context(RwSignal::<Option<(Uuid, u64)>>::new(None));
    crate::websocket_client::use_websocket();

    // Hoisted above <Router> so these survive client-side navigation instead
    // of being torn down and recreated by every page's own <MainLayout>
    // (each page wraps its own <MainLayout>, so the sidebar remounts on
    // every route change). Fixes the sidebar's Logout->Login and "Loading
    // games..." flash: the resources themselves never remount, only the
    // components reading them do, so a fresh mount just reads the value
    // already sitting in these signals instead of starting from None.
    let logout_action = ServerAction::<crate::auth::Logout>::new();
    provide_context(logout_action);

    let active_games: LocalResource<
        Result<Vec<crate::game::server_fns::GameSummary>, ServerFnError>,
    > = LocalResource::new(move || async move {
        let _ = last_update.get();
        crate::game::server_fns::get_active_games().await
    });
    provide_context(active_games);

    // None until the fetch resolves; treat that as logged-out so anonymous
    // visitors never see "Logout". Re-fetches after a logout dispatch.
    let current_user: LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>> =
        LocalResource::new(move || async move {
            let _ = logout_action.version().get();
            crate::auth::get_current_user().await
        });
    provide_context(current_user);

    view! {
        <Stylesheet id="leptos" href="/pkg/web.css"/>
        <Title text="brdg.me"/>

        <Router>
            <Routes fallback=|| "Page not found.".into_view()>
                <Route path=StaticSegment("") view=HomePage/>
                <Route path=StaticSegment("login") view=LoginPage/>
                <Route path=StaticSegment("games") view=GamesPage/>
                <Route path=StaticSegment("dashboard") view=DashboardPage/>
                <Route path=(StaticSegment("games"), ParamSegment("id")) view=GamePage/>
            </Routes>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <MainLayout>
            <h1>"Welcome to brdg.me"</h1>
            <p>"Lo-fi board games by email and web."</p>
            <A href="/dashboard">"Go to Dashboard"</A>
        </MainLayout>
    }
}

#[component]
fn LoginPage() -> impl IntoView {
    let (show_code_input, set_show_code_input) = signal(false);
    let (email, set_email) = signal(String::new());

    let email_input = NodeRef::<html::Input>::new();
    let code_email_input = NodeRef::<html::Input>::new();
    let code_input = NodeRef::<html::Input>::new();

    let login_action = Action::new(|email: &String| {
        let email = email.clone();
        async move { login(email).await }
    });

    let confirm_action = Action::new(|(email, token): &(String, String)| {
        let email = email.clone();
        let token = token.clone();
        async move { confirm_login(email, token).await }
    });

    // Show code input once server confirms email was sent.
    Effect::new(move |_| {
        if let Some(Ok(resp)) = login_action.value().get()
            && resp.success
        {
            set_show_code_input.set(true);
        }
    });

    // Navigate to dashboard on successful login.
    let navigate = use_navigate();
    Effect::new(move |_| {
        if confirm_action.value().get().is_some_and(|r| r.is_ok()) {
            navigate("/dashboard", NavigateOptions::default());
        }
    });

    let on_email_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(email_value) = email_input.get().map(|el| el.value()) else {
            leptos::logging::warn!("on_email_submit: email_input not mounted");
            return;
        };
        set_email.set(email_value.clone());
        login_action.dispatch(email_value);
    };

    let on_code_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(token) = code_input.get().map(|el| el.value()) else {
            leptos::logging::warn!("on_code_submit: code_input not mounted");
            return;
        };
        // If the email wasn't already collected (the "I already have a login
        // code" shortcut skips step 1), read it from the code-step email
        // field instead.
        let email_value = if email.get_untracked().is_empty() {
            let Some(email_value) = code_email_input.get().map(|el| el.value()) else {
                leptos::logging::warn!("on_code_submit: code_email_input not mounted");
                return;
            };
            set_email.set(email_value.clone());
            email_value
        } else {
            email.get_untracked()
        };
        confirm_action.dispatch((email_value, token));
    };

    let show_code_link = move |_| {
        set_show_code_input.set(true);
    };

    view! {
        <div class="login">
            <h1>"brdg.me"</h1>
            <div class="subtitle">"Lo-fi board games, email / web"</div>

            <Show when=move || !show_code_input.get()>
                <div>
                    <div>"Enter your email address to start"</div>
                    <form on:submit=on_email_submit>
                        <div>
                            <input
                                type="email"
                                node_ref=email_input
                                placeholder="Email address"
                                required
                                disabled=move || login_action.pending().get()
                            />
                            <input
                                type="submit"
                                value="Get code"
                                disabled=move || login_action.pending().get()
                            />
                        </div>
                        <Show when=move || login_action.pending().get()>
                            <div class="spinner">
                                <div class="bounce1"></div>
                                <div class="bounce2"></div>
                                <div class="bounce3"></div>
                            </div>
                        </Show>
                        <div class="hasCode">
                            <a on:click=show_code_link style="cursor:pointer">"I already have a login code"</a>
                        </div>
                    </form>
                    <Show when=move || login_action.value().get().is_some_and(|r| r.is_err())>
                        <div class="error">"Failed to send login email. Please try again."</div>
                    </Show>
                </div>
            </Show>

            <Show when=move || show_code_input.get()>
                <div>
                    <Show when=move || !email.get().is_empty()>
                        <div>"Logging in as " <a on:click=move |_| set_show_code_input.set(false) style="cursor:pointer">{email.get()}</a></div>
                    </Show>
                    <div>
                        <div>"A login code has been sent to your email, please enter it here"</div>
                        <form on:submit=on_code_submit>
                            <Show when=move || email.get().is_empty()>
                                <input
                                    type="email"
                                    node_ref=code_email_input
                                    placeholder="Email address"
                                    required
                                />
                            </Show>
                            <input
                                type="tel"
                                pattern="[0-9]*"
                                node_ref=code_input
                                placeholder="Login code"
                                required
                            />
                            <input type="submit" value="Play!"/>
                        </form>
                        <Show when=move || confirm_action.value().get().is_some_and(|r| r.is_err())>
                            <div class="error">"Invalid or expired code. Please try again."</div>
                        </Show>
                    </div>
                </div>
            </Show>
        </div>
    }
}

/// Per-opponent slot state: either a human (email) or a bot (name + difficulty).
#[derive(Debug, Clone)]
enum OpponentSlot {
    Human(String),
    Bot { name: String, difficulty: String },
}

impl Default for OpponentSlot {
    fn default() -> Self {
        OpponentSlot::Human(String::new())
    }
}

#[component]
fn GamesPage() -> impl IntoView {
    use crate::game::server_fns::{BotSlot, create_new_game, get_available_game_types};

    let game_types = LocalResource::new(get_available_game_types);

    let (selected_type_id, set_selected_type_id) = signal(None::<Uuid>);
    let (selected_version_id, set_selected_version_id) = signal(None::<Uuid>);
    let (player_count, set_player_count) = signal(2i32);
    let (opponent_slots, set_opponent_slots) = signal(vec![OpponentSlot::default()]);

    // Initialize selections when game types first load.
    Effect::new(move |_| {
        if let Some(Ok(types)) = game_types.get()
            && selected_type_id.get_untracked().is_none()
            && let Some(first) = types.first()
        {
            set_selected_type_id.set(Some(first.id));
            set_selected_version_id.set(first.versions.first().map(|v| v.id));
            set_player_count.set(first.player_counts.first().copied().unwrap_or(2));
        }
    });

    // Resize opponent slot list when player count changes.
    Effect::new(move |_| {
        let n = (player_count.get() - 1).max(0) as usize;
        set_opponent_slots.update(|v| v.resize_with(n, OpponentSlot::default));
    });

    let create_action = Action::new(
        |(version_id, emails, bots): &(Uuid, Vec<String>, Vec<BotSlot>)| {
            let version_id = *version_id;
            let emails = if emails.is_empty() {
                None
            } else {
                Some(emails.clone())
            };
            let bots = if bots.is_empty() {
                None
            } else {
                Some(bots.clone())
            };
            async move { create_new_game(version_id, emails, bots).await }
        },
    );

    let navigate = use_navigate();
    Effect::new(move |_| {
        if let Some(Ok(id)) = create_action.value().get() {
            navigate(&format!("/games/{}", id), NavigateOptions::default());
        }
    });

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if let Some(version_id) = selected_version_id.get_untracked() {
            let slots = opponent_slots.get_untracked();
            let mut emails = Vec::new();
            let mut bots = Vec::new();
            for slot in slots {
                match slot {
                    OpponentSlot::Human(email) => emails.push(email),
                    OpponentSlot::Bot { name, difficulty } => {
                        bots.push(BotSlot { name, difficulty })
                    }
                }
            }
            create_action.dispatch((version_id, emails, bots));
        }
    };

    view! {
        <MainLayout>
            <div class="new-game">
                <h1>"New Game"</h1>
                {move || match game_types.get() {
                    None => view! { <p>"Loading..."</p> }.into_any(),
                    Some(Err(e)) => view! { <p class="error">"Error: " {e.to_string()}</p> }.into_any(),
                    Some(Ok(t)) if t.is_empty() => view! { <p>"No games available."</p> }.into_any(),
                    Some(Ok(types)) => {
                        let types = StoredValue::new(types);
                        view! {
                            <form on:submit=on_submit>
                                <div class="form-row">
                                    <label>"Game"</label>
                                    <select on:change=move |ev| {
                                        if let Ok(id) = event_target_value(&ev).parse::<Uuid>()
                                            && let Some(gt) = types.with_value(|t| t.iter().find(|g| g.id == id).cloned()) {
                                                set_selected_type_id.set(Some(id));
                                                set_selected_version_id.set(gt.versions.first().map(|v| v.id));
                                                set_player_count.set(gt.player_counts.first().copied().unwrap_or(2));
                                            }
                                    }>
                                        {types.with_value(|t| t.iter().map(|gt| {
                                            let id = gt.id.to_string();
                                            let name = gt.name.clone();
                                            view! { <option value=id>{name}</option> }
                                        }).collect_view())}
                                    </select>
                                </div>

                                {move || types.with_value(|t| {
                                    t.iter().find(|gt| Some(gt.id) == selected_type_id.get()).map(|gt| {
                                        let version_row = if gt.versions.len() > 1 {
                                            let versions = gt.versions.clone();
                                            view! {
                                                <div class="form-row">
                                                    <label>"Version"</label>
                                                    <select on:change=move |ev| {
                                                        set_selected_version_id.set(event_target_value(&ev).parse::<Uuid>().ok());
                                                    }>
                                                        {versions.iter().map(|v| {
                                                            let id = v.id.to_string();
                                                            let name = v.name.clone();
                                                            view! { <option value=id>{name}</option> }
                                                        }).collect_view()}
                                                    </select>
                                                </div>
                                            }.into_any()
                                        } else {
                                            ().into_any()
                                        };

                                        let count_row = if gt.player_counts.len() > 1 {
                                            let counts = gt.player_counts.clone();
                                            view! {
                                                <div class="form-row">
                                                    <label>"Players"</label>
                                                    <select on:change=move |ev| {
                                                        if let Ok(n) = event_target_value(&ev).parse::<i32>() {
                                                            set_player_count.set(n);
                                                        }
                                                    }>
                                                        {counts.iter().map(|&n| {
                                                            view! { <option value=n.to_string()>{n}</option> }
                                                        }).collect_view()}
                                                    </select>
                                                </div>
                                            }.into_any()
                                        } else {
                                            ().into_any()
                                        };

                                        view! {
                                            {version_row}
                                            {count_row}
                                        }.into_any()
                                    })
                                })}

                                {move || {
                                    let n = (player_count.get() - 1).max(0) as usize;
                                    (0..n).map(|i| {
                                        let slot = move || opponent_slots.get().get(i).cloned().unwrap_or_default();
                                        let is_bot = move || matches!(slot(), OpponentSlot::Bot { .. });
                                        view! {
                                            <div class="form-row">
                                                <label>"Opponent " {i + 1}</label>
                                                <select on:change=move |ev| {
                                                    let val = event_target_value(&ev);
                                                    set_opponent_slots.update(|v| {
                                                        if let Some(s) = v.get_mut(i) {
                                                            *s = if val == "bot" {
                                                                OpponentSlot::Bot { name: format!("Bot {}", i + 1), difficulty: "medium".to_string() }
                                                            } else {
                                                                OpponentSlot::Human(String::new())
                                                            };
                                                        }
                                                    });
                                                }>
                                                    <option value="human" selected=move || !is_bot()>"Human"</option>
                                                    <option value="bot" selected=is_bot>"Bot"</option>
                                                </select>
                                            </div>
                                            <Show when=move || !is_bot()>
                                                <div class="form-row">
                                                    <label>"Email"</label>
                                                    <input
                                                        type="email"
                                                        placeholder="Email address"
                                                        required
                                                        prop:value=move || match slot() { OpponentSlot::Human(e) => e, _ => String::new() }
                                                        on:input=move |ev| {
                                                            let val = event_target_value(&ev);
                                                            set_opponent_slots.update(|v| {
                                                                if let Some(s) = v.get_mut(i) {
                                                                    *s = OpponentSlot::Human(val);
                                                                }
                                                            });
                                                        }
                                                    />
                                                </div>
                                            </Show>
                                            <Show when=move || is_bot()>
                                                <div class="form-row">
                                                    <label>"Bot name"</label>
                                                    <input
                                                        type="text"
                                                        placeholder="Bot name"
                                                        required
                                                        prop:value=move || match slot() { OpponentSlot::Bot { name, .. } => name, _ => String::new() }
                                                        on:input=move |ev| {
                                                            let val = event_target_value(&ev);
                                                            set_opponent_slots.update(|v| {
                                                                if let Some(OpponentSlot::Bot { name, .. }) = v.get_mut(i) {
                                                                    *name = val;
                                                                }
                                                            });
                                                        }
                                                    />
                                                </div>
                                                <div class="form-row">
                                                    <label>"Difficulty"</label>
                                                    <select on:change=move |ev| {
                                                        let val = event_target_value(&ev);
                                                        set_opponent_slots.update(|v| {
                                                            if let Some(OpponentSlot::Bot { difficulty, .. }) = v.get_mut(i) {
                                                                *difficulty = val;
                                                            }
                                                        });
                                                    }>
                                                        <option value="easy">"Easy"</option>
                                                        <option value="medium" selected=true>"Medium"</option>
                                                        <option value="hard">"Hard"</option>
                                                    </select>
                                                </div>
                                            </Show>
                                        }
                                    }).collect_view()
                                }}

                                <div class="form-row">
                                    <input
                                        type="submit"
                                        value="Create Game"
                                        disabled=move || create_action.pending().get()
                                    />
                                </div>

                                <Show when=move || create_action.value().get().is_some_and(|r| r.is_err())>
                                    <div class="error">
                                        {move || create_action.value().get()
                                            .and_then(|r| r.err())
                                            .map(|e| e.to_string())
                                            .unwrap_or_default()}
                                    </div>
                                </Show>
                            </form>
                        }.into_any()
                    }
                }}
            </div>
        </MainLayout>
    }
}

#[component]
fn DashboardPage() -> impl IntoView {
    view! {
        <MainLayout>
            <h1>"Dashboard"</h1>
            <p>"View your active games and statistics."</p>

            <div class="dashboard-sections">
                <section class="active-games">
                    <h2>"Active Games"</h2>
                    <p>"Use the sidebar to navigate your games."</p>
                </section>
            </div>
        </MainLayout>
    }
}

#[component]
fn GamePage() -> impl IntoView {
    use crate::components::game::*;
    use crate::game::server_fns::{get_game_details, mark_read};
    use std::str::FromStr;

    let params = leptos_router::hooks::use_params_map();
    let game_id = move || {
        params
            .get()
            .get("id")
            .as_deref()
            .and_then(|id| Uuid::from_str(id).ok())
    };

    let game_update = expect_context::<RwSignal<Option<(Uuid, u64)>>>();

    // Per-game sequence number, isolated from other games' WS updates so this
    // page's resources don't refetch when a different game changes.
    let seq_for_this_game = Memo::new(move |_| {
        let current_id = game_id();
        game_update
            .get()
            .and_then(|(id, seq)| (Some(id) == current_id).then_some(seq))
    });

    // Call mark_read on mount and whenever the game ID changes.
    Effect::new(move |_| {
        if let Some(id) = game_id() {
            leptos::task::spawn_local(async move {
                let _ = mark_read(id).await;
            });
        }
    });

    // Blocking so SSR waits for data and serialises it to the client, avoiding
    // a second fetch on hydration and preventing the stuck-loading state on
    // hard refresh. Re-keyed on the per-game WS sequence memo, which isolates
    // this page's refetches to this game's WS signals.
    let game_data = Resource::new_blocking(
        move || (game_id(), seq_for_this_game.get()),
        |(id, _)| async move {
            match id {
                Some(id) => get_game_details(id).await,
                None => Err(ServerFnError::new("Invalid Game ID")),
            }
        },
    );

    // is_my_turn starts false in hydrate mode (Memo returns false until resource
    // deserializes). This only changes a CSS class — no structural mismatch.
    let is_my_turn = Memo::new(move |_| {
        game_data
            .get()
            .and_then(|r| r.ok())
            .map(|d| d.is_my_turn)
            .unwrap_or(false)
    });

    // MainLayout is outside Transition so it is always in the initial SSR
    // HTML with no streaming placeholder risk. Transition (not Suspense)
    // wraps the game content: Suspense's fallback replaces its children on
    // every refetch, blanking the board to white on each WS-triggered
    // update; Transition keeps the last-rendered children visible during a
    // refetch and only shows `fallback` before the first load.
    view! {
        <MainLayout
            is_my_turn=Signal::from(is_my_turn)
            has_sub_menu=Signal::from(true)
            has_next_game=Signal::from(is_my_turn)
        >
            <Transition fallback=|| view! { <div></div> }>
                {move || {
                    let base = game_data.get();
                    base.map(|res| match res {
                        Err(e) => view! { <div class="error">"Error: " {e.to_string()}</div> }.into_any(),
                        Ok(data) => {
                            let is_turn = data.is_my_turn;
                            let is_finished = data.is_finished;
                            let id = data.id;
                            let html = data.html.clone();
                            let command_spec = data.command_spec.clone();
                            let player_names: Vec<String> = data.players.iter().map(|p| p.name.clone()).collect();
                            let waiting_on = StoredValue::new(
                                data.players.iter()
                                    .filter(|p| p.is_turn)
                                    .map(|p| (p.name.clone(), p.color.clone()))
                                    .collect::<Vec<_>>()
                            );
                            view! {
                                <div class="game-container">
                                    <div class="game-main">
                                        <GameBoard html=html />
                                        <RecentGameLogs game_id=id />
                                        <Show when=move || is_turn>
                                            <GameCommandInput
                                                game_id=id
                                                command_spec=command_spec.clone()
                                                player_names=player_names.clone()
                                            />
                                        </Show>
                                        <Show when=move || !is_turn && !is_finished>
                                            <div class="game-current-turn">
                                                "Waiting on: "
                                                {waiting_on.with_value(|w| w.iter().enumerate().map(|(i, (name, color))| {
                                                    let name = name.clone();
                                                    let color = color.clone();
                                                    view! {
                                                        <span>
                                                            {if i > 0 { ", " } else { "" }}
                                                            <PlayerName name=name color=color />
                                                        </span>
                                                    }
                                                }).collect_view())}
                                            </div>
                                        </Show>
                                    </div>
                                    <GameMeta data=data />
                                </div>
                            }.into_any()
                        },
                    })
                }}
            </Transition>
        </MainLayout>
    }
}
