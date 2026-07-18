use leptos::html;
use leptos::prelude::*;
use leptos_meta::{HashedStylesheet, MetaTags, Title, provide_meta_context};
use leptos_router::{
    NavigateOptions, ParamSegment, StaticSegment,
    components::{A, Route, Router, Routes},
    hooks::use_navigate,
};
use uuid::Uuid;

use crate::auth::server::{confirm_login, login};
use crate::components::MainLayout;

// Reads the `theme` cookie before first paint so the correct theme is
// visible from the very first frame, both logged out and hard-refreshed
// while logged in (no flash of the wrong theme). Absent/"system" removes the
// attribute so the `prefers-color-scheme` media query in THEME_STYLE_CSS
// takes over. An explicit `data-theme` set by this script always wins.
// The slug list here must stay in sync with `crate::theme::THEME_SLUGS`;
// `theme_boot_script_contains_all_theme_slugs` (below) pins that.
const THEME_BOOT_SCRIPT: &str = r#"(function(){try{var m=document.cookie.match(/(?:^|; )theme=([^;]*)/);var t=m?decodeURIComponent(m[1]):null;if(t&&["brdgme-light","brdgme-dark","dracula","alucard","solarized-dark","solarized-light","nord-dark","nord-light","one-dark","one-light","gruvbox-dark","gruvbox-light","catppuccin-mocha","catppuccin-latte","tokyo-night","tokyo-night-storm","tokyo-night-light","night-owl","light-owl","synthwave-84","papercolor-light","papercolor-dark","monokai","darcula","vs-code-dark-plus","vs-code-dark-modern","brdgme-light-deuteranopia","brdgme-light-protanopia","brdgme-light-tritanopia","brdgme-dark-deuteranopia","brdgme-dark-protanopia","brdgme-dark-tritanopia","modus-operandi-tritanopia","modus-vivendi-tritanopia"].indexOf(t)>=0){document.documentElement.dataset.theme=t;}else{delete document.documentElement.dataset.theme;}}catch(e){}})();"#;

// Scrubs cookies and auth headers before an event leaves the browser -
// defensive (doesn't throw if `event.request`/`headers` are absent) since
// this runs on every captured error.
const SENTRY_BEFORE_SEND_JS: &str = r#"function(event){try{if(event.request){delete event.request.cookies;if(event.request.headers){Object.keys(event.request.headers).forEach(function(k){if(k.toLowerCase()==="authorization"||k.toLowerCase()==="cookie"){delete event.request.headers[k];}});}}}catch(e){}return event;}"#;

// `shell()` compiles into both the ssr-feature server binary and the
// hydrate-feature wasm binary (the crate builds twice under cargo-leptos).
// Client-side JS never calls `shell()` (see lib.rs's `hydrate()`, which
// calls `hydrate_body` directly), so the hydrate build's `None` stub is dead
// code - gating it explicitly keeps `std::env::var` reads confined to ssr,
// matching every other env-var read in this crate (see e.g. auth/session.rs,
// main.rs).
#[cfg(feature = "ssr")]
fn sentry_dsn_and_release() -> Option<(String, Option<String>)> {
    let dsn = std::env::var("SENTRY_DSN_WEB").ok()?;
    let release = std::env::var("SENTRY_RELEASE").ok();
    Some((dsn, release))
}

#[cfg(not(feature = "ssr"))]
fn sentry_dsn_and_release() -> Option<(String, Option<String>)> {
    None
}

/// Escapes a string for embedding in a double-quoted JS string literal.
/// DSNs/release strings are not expected to contain quotes, but this is
/// cheap enough to apply unconditionally rather than assume that.
fn js_string_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn sentry_init_snippet(dsn: &str, release: Option<&str>) -> String {
    let release_field = release
        .map(|r| format!(r#","release":"{}""#, js_string_escape(r)))
        .unwrap_or_default();
    // No `tracesSampleRate` key: omitting it (not setting it to 0) disables
    // the tracing integration entirely, protecting the free-tier error
    // quota - see docs/superpowers/specs/2026-07-15-wasm-prod-errors-design.md.
    format!(
        r#"window.Sentry.init({{"dsn":"{}","integrations":[window.SentryWasmIntegration()],"sendDefaultPii":false{},"beforeSend":{}}});"#,
        js_string_escape(dsn),
        release_field,
        SENTRY_BEFORE_SEND_JS,
    )
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    let theme_css = crate::theme::THEME_STYLE_CSS.clone();
    // Unset SENTRY_DSN_WEB -> neither script tag is emitted, so dev/Tilt is
    // completely unaffected.
    let sentry_snippet = sentry_dsn_and_release()
        .map(|(dsn, release)| sentry_init_snippet(&dsn, release.as_deref()));
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="apple-mobile-web-app-capable" content="yes"/>
                <meta name="mobile-web-app-capable" content="yes"/>
                <link rel="icon" type="image/svg+xml" href="/favicon.svg"/>
                <style inner_html=theme_css></style>
                <script inner_html=THEME_BOOT_SCRIPT></script>
                {sentry_snippet.map(|snippet| view! {
                    <script src="/sentry.js"></script>
                    <script inner_html=snippet></script>
                })}
                <HashedStylesheet options=options.clone() id="leptos"/>
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

    // Profile theme sync: once the current user resolves to logged-in for
    // the first time this session, fetch their stored theme (if any) and
    // apply it - the profile wins over whatever was showing pre-login
    // (system default or a locally-set-but-unsaved theme). If the profile has
    // no stored preference, instead push the local choice (if any) up to the
    // profile, so the local choice syncs to the account and follows the user
    // to new devices. No-ops for anonymous visitors. Runs only on hydrate
    // (Effects are inert during SSR), so `set_theme_client`'s/`web_sys` calls
    // are safe here.
    let applied_profile_theme = RwSignal::new(false);
    Effect::new(move |_| {
        if matches!(current_user.get(), Some(Ok(Some(_)))) && !applied_profile_theme.get_untracked()
        {
            applied_profile_theme.set(true);
            leptos::task::spawn_local(async move {
                match crate::auth::get_user_theme().await {
                    Ok(Some(theme)) => set_theme_client(Some(&theme)),
                    Ok(None) => {
                        if let Some(local) = local_data_theme()
                            && crate::theme::is_known_slug(&local)
                        {
                            let _ = crate::auth::set_theme(Some(local)).await;
                        }
                    }
                    Err(_) => {}
                }
            });
        }
    });

    // Derived from the same active-games data the sidebar renders, not a
    // new query - counts games where it's this user's turn.
    let turn_count = Memo::new(move |_| {
        active_games
            .get()
            .and_then(|r| r.ok())
            .map(|games| count_my_turn(&games))
            .unwrap_or(0)
    });
    let title_text = move || {
        let n = turn_count.get();
        if n > 0 {
            format!("brdg.me ({n})")
        } else {
            "brdg.me".to_string()
        }
    };

    view! {
        <Title text=title_text/>

        <Router>
            <Routes fallback=|| "Page not found.".into_view()>
                <Route path=StaticSegment("") view=HomePage/>
                <Route path=StaticSegment("login") view=LoginPage/>
                <Route path=StaticSegment("games") view=GamesPage/>
                <Route path=StaticSegment("dashboard") view=DashboardPage/>
                <Route path=StaticSegment("settings") view=crate::settings::SettingsPage/>
                <Route path=StaticSegment("friends") view=crate::friends::FriendsPage/>
                <Route path=(StaticSegment("games"), ParamSegment("id")) view=GamePage/>
            </Routes>
        </Router>
    }
}

/// Applies a theme selection instantly client-side: sets/removes
/// `document.documentElement.dataset.theme` (picked up immediately by the
/// `[data-theme="..."]` CSS from `THEME_STYLE_CSS`) and persists the choice
/// in the `theme` cookie (`None` -> "System" -> delete the cookie) so a hard
/// refresh or a future visit boots into the same theme via `THEME_BOOT_SCRIPT`.
/// No-op if `web_sys::window()` is unavailable (SSR - callers only invoke
/// this from Effects/event handlers, which don't run during SSR).
pub(crate) fn set_theme_client(slug: Option<&str>) {
    use web_sys::wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    if let Some(html) = document.document_element() {
        match slug {
            Some(s) => {
                let _ = html.set_attribute("data-theme", s);
            }
            None => {
                let _ = html.remove_attribute("data-theme");
            }
        }
    }
    if let Ok(html_doc) = document.dyn_into::<web_sys::HtmlDocument>() {
        let cookie = match slug {
            Some(s) => format!("theme={}; path=/; max-age=31536000", s),
            None => "theme=; path=/; max-age=0".to_string(),
        };
        let _ = html_doc.set_cookie(&cookie);
    }
}

/// Reads the current `data-theme` attribute set on `<html>` (by
/// `THEME_BOOT_SCRIPT` pre-paint, or by a prior `set_theme_client` call this
/// session). `None` if unset (SSR, or "system") or `web_sys` is unavailable.
pub(crate) fn local_data_theme() -> Option<String> {
    web_sys::window()?
        .document()?
        .document_element()?
        .get_attribute("data-theme")
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <MainLayout>
            <div class="content-page">
                <h1>"Welcome to brdg.me"</h1>
                <p>"Lo-fi board games by email and web."</p>
                <A href="/dashboard">"Go to Dashboard"</A>
            </div>
        </MainLayout>
    }
}

#[component]
fn LoginPage() -> impl IntoView {
    let current_user =
        expect_context::<LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>>();
    let active_games = expect_context::<
        LocalResource<Result<Vec<crate::game::server_fns::GameSummary>, ServerFnError>>,
    >();

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

    // Navigate to dashboard on successful login. `current_user`/`active_games`
    // live above the Router (see App()), so this navigation no longer
    // remounts them - refetch explicitly or the sidebar keeps showing
    // logged-out state and no active games until a hard reload.
    let navigate = use_navigate();
    Effect::new(move |_| {
        if confirm_action.value().get().is_some_and(|r| r.is_ok()) {
            current_user.refetch();
            active_games.refetch();
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

    // Autofocus: email field on load, code field once the code step renders
    // (both re-fire once their `NodeRef` resolves, matching the pattern
    // already used by `GameCommandInput`'s mount effect).
    Effect::new(move |_| {
        if let Some(el) = email_input.get() {
            let _ = el.focus();
        }
    });
    Effect::new(move |_| {
        if show_code_input.get()
            && let Some(el) = code_input.get()
        {
            let _ = el.focus();
        }
    });

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
                            <crate::components::Spinner/>
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

/// Per-opponent slot state: a human (free-text email), a known user picked
/// from a suggestion chip (#30 D6), or a bot (name + difficulty).
#[derive(Debug, Clone)]
enum OpponentSlot {
    Human(String),
    KnownUser { id: Uuid, name: String },
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
    let suggestions = LocalResource::new(crate::friends::get_opponent_suggestions);

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
        |(version_id, ids, emails, bots): &(Uuid, Vec<Uuid>, Vec<String>, Vec<BotSlot>)| {
            let version_id = *version_id;
            let ids = if ids.is_empty() {
                None
            } else {
                Some(ids.clone())
            };
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
            async move { create_new_game(version_id, ids, emails, bots).await }
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
            let mut ids = Vec::new();
            let mut emails = Vec::new();
            let mut bots = Vec::new();
            for slot in slots {
                match slot {
                    OpponentSlot::Human(email) => emails.push(email),
                    OpponentSlot::KnownUser { id, .. } => ids.push(id),
                    OpponentSlot::Bot { name, difficulty } => {
                        bots.push(BotSlot { name, difficulty })
                    }
                }
            }
            create_action.dispatch((version_id, ids, emails, bots));
        }
    };

    view! {
        <MainLayout>
            <div class="new-game content-page">
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
                                                {move || match slot() {
                                                    OpponentSlot::KnownUser { name, .. } => view! {
                                                        <div class="form-row">
                                                            <label>"Player"</label>
                                                            <span class="chip chip-selected">
                                                                {name}
                                                                " "
                                                                <a href="#" on:click=move |ev| {
                                                                    ev.prevent_default();
                                                                    set_opponent_slots.update(|v| {
                                                                        if let Some(s) = v.get_mut(i) {
                                                                            *s = OpponentSlot::Human(String::new());
                                                                        }
                                                                    });
                                                                }>"x"</a>
                                                            </span>
                                                        </div>
                                                    }.into_any(),
                                                    _ => view! {
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
                                                        <div class="form-row chip-row">
                                                            {move || {
                                                                let taken: Vec<Uuid> = opponent_slots.get().iter().filter_map(|s| match s {
                                                                    OpponentSlot::KnownUser { id, .. } => Some(*id),
                                                                    _ => None,
                                                                }).collect();
                                                                match suggestions.get() {
                                                                    Some(Ok(sugs)) if !sugs.is_empty() => sugs.iter()
                                                                        .filter(|s| !taken.contains(&s.user_id))
                                                                        .map(|s| {
                                                                            let id = s.user_id;
                                                                            let name = s.name.clone();
                                                                            let label = name.clone();
                                                                            view! {
                                                                                <a href="#" class="chip" class:chip-friend=s.is_friend on:click=move |ev| {
                                                                                    ev.prevent_default();
                                                                                    set_opponent_slots.update(|v| {
                                                                                        if let Some(slot) = v.get_mut(i) {
                                                                                            *slot = OpponentSlot::KnownUser { id, name: name.clone() };
                                                                                        }
                                                                                    });
                                                                                }>{label}</a>
                                                                            }
                                                                        }).collect_view().into_any(),
                                                                    _ => ().into_any(),
                                                                }
                                                            }}
                                                        </div>
                                                    }.into_any(),
                                                }}
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
    use crate::friends::{RespondToFriendRequest, get_friend_activity, get_friends_overview};

    let (refresh, set_refresh) = signal(0u32);
    let overview = LocalResource::new(move || {
        refresh.track();
        get_friends_overview()
    });
    let activity = LocalResource::new(get_friend_activity);
    let respond_action = ServerAction::<RespondToFriendRequest>::new();
    Effect::new(move |_| {
        if let Some(Ok(())) = respond_action.value().get() {
            set_refresh.update(|n| *n += 1);
        }
    });

    view! {
        <MainLayout>
            <div class="content-page">
                <h1>"Dashboard"</h1>

                <div class="dashboard-sections">
                    <section class="friend-requests">
                        <h2>"Friend requests"</h2>
                        {move || match overview.get() {
                            None => view! { <p>"Loading..."</p> }.into_any(),
                            Some(Err(_)) => ().into_any(), // anonymous or error: hide
                            Some(Ok(o)) if o.incoming.is_empty() =>
                                view! { <p>"No pending requests."</p> }.into_any(),
                            Some(Ok(o)) => o.incoming.iter().map(|r| {
                                let id = r.request_id;
                                let name = r.name.clone();
                                view! {
                                    <div class="friend-row">
                                        <span>{name}</span>
                                        " "
                                        <a href="#" on:click=move |ev| {
                                            ev.prevent_default();
                                            respond_action.dispatch(RespondToFriendRequest { request_id: id, accept: true, block: false });
                                        }>"Accept"</a>
                                        " | "
                                        <a href="#" on:click=move |ev| {
                                            ev.prevent_default();
                                            respond_action.dispatch(RespondToFriendRequest { request_id: id, accept: false, block: false });
                                        }>"Decline"</a>
                                        " | "
                                        <a href="#" on:click=move |ev| {
                                            ev.prevent_default();
                                            let confirmed = web_sys::window()
                                                .and_then(|w| w.confirm_with_message("Decline and block? They will no longer be able to send you friend requests or add you to games.").ok())
                                                .unwrap_or(false);
                                            if confirmed {
                                                respond_action.dispatch(RespondToFriendRequest { request_id: id, accept: false, block: true });
                                            }
                                        }>"Decline and block"</a>
                                    </div>
                                }
                            }).collect_view().into_any(),
                        }}
                        <p><a href="/friends">"Manage friends"</a></p>
                    </section>

                    <section class="friends-active-games">
                        <h2>"Friends' active games"</h2>
                        {move || match activity.get() {
                            None => view! { <p>"Loading..."</p> }.into_any(),
                            Some(Err(_)) => ().into_any(),
                            Some(Ok(a)) if a.active.is_empty() =>
                                view! { <p>"No games to watch right now."</p> }.into_any(),
                            Some(Ok(a)) => a.active.iter().map(|g| {
                                let href = format!("/games/{}", g.game_id);
                                let label = format!("{}: {}", g.game_type, g.player_names.join(", "));
                                view! { <div><a href=href>{label}</a></div> }
                            }).collect_view().into_any(),
                        }}
                    </section>

                    <section class="friends-recent-results">
                        <h2>"Friends' recent results"</h2>
                        {move || match activity.get() {
                            None => view! { <p>"Loading..."</p> }.into_any(),
                            Some(Err(_)) => ().into_any(),
                            Some(Ok(a)) if a.results.is_empty() =>
                                view! { <p>"No recent results."</p> }.into_any(),
                            Some(Ok(a)) => a.results.iter().map(|g| {
                                let href = format!("/games/{}", g.game_id);
                                let players = g.player_names.iter().zip(g.places.iter())
                                    .map(|(n, p)| if *p > 0 { format!("{p}. {n}") } else { n.clone() })
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                let label = format!("{}: {}", g.game_type, players);
                                view! { <div><a href=href>{label}</a></div> }
                            }).collect_view().into_any(),
                        }}
                    </section>

                    <section class="active-games">
                        <h2>"Active Games"</h2>
                        <p>"Use the sidebar to navigate your games."</p>
                    </section>
                </div>
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

    // Per-game sequence state, isolated from other games' WS updates so this
    // page's resources don't refetch when a different game changes. Holds
    // (game_id, last seq for it) so an update for another game leaves the
    // memo value unchanged (PartialEq dedupe) instead of flipping to None.
    let seq_for_this_game = Memo::new(move |prev: Option<&(Option<Uuid>, Option<u64>)>| {
        track_game_seq(prev.copied(), game_id(), game_update.get())
    });

    // Just the game id, deduped: changes only when navigating to a
    // different game, never on seq-only WS/command refetches. Keying the
    // <Transition> below on this remounts it per game, so its spinner
    // fallback shows while a newly navigated-to game loads, while
    // refetches of the current game keep the stale board (no spinner).
    let current_game = Memo::new(move |_| game_id());

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
        move || seq_for_this_game.get(),
        |(id, _)| async move {
            match id {
                Some(id) => get_game_details(id).await,
                None => Err(ServerFnError::new("Invalid Game ID")),
            }
        },
    );

    // Hoisted above the <Transition> closure below so it survives the
    // remount that closure does on every `game_data` refetch (i.e. every
    // command submit). GameLogs and RecentGameLogs used to each create their
    // own LocalResource for this, so remounting reset them to None and the
    // logs flashed blank until the refetch resolved. Shared via context
    // instead so a fresh mount just reads the value already sitting here.
    let logs: LocalResource<Result<Vec<crate::game::server_fns::GameLogEntry>, ServerFnError>> =
        LocalResource::new(move || async move {
            let id = game_id();
            let _ = seq_for_this_game.get();
            match id {
                Some(id) => crate::game::server_fns::get_game_logs(id).await,
                None => Err(ServerFnError::new("Invalid Game ID")),
            }
        });
    provide_context(logs);

    // Hoisted for the same reason as `logs` above: the <Transition> closure
    // remounts GameCommandInput on every game_data refetch, and a local
    // signal there would reset typed-but-unsent text to "" each time.
    let command_text = crate::components::game::CommandInputText(RwSignal::new(String::new()));
    provide_context(command_text);

    // Typed text must not leak between games when navigating game-to-game
    // (the route component is reused, so nothing else resets it).
    Effect::new(move |prev: Option<Option<Uuid>>| {
        let id = game_id();
        if let Some(prev_id) = prev
            && prev_id != id
        {
            command_text.0.set(String::new());
        }
        id
    });

    // MainLayout is outside Transition so it is always in the initial SSR
    // HTML with no streaming placeholder risk. Transition (not Suspense)
    // wraps the game content: Suspense's fallback replaces its children on
    // every refetch, blanking the board to white on each WS-triggered
    // update; Transition keeps the last-rendered children visible during a
    // refetch and only shows `fallback` before the first load.
    view! {
        <MainLayout has_sub_menu=Signal::from(true)>
            {move || {
                current_game.track();
                view! {
                    <Transition fallback=|| view! {
                        <div class="game-loading"><crate::components::Spinner/></div>
                    }>
                        {move || {
                            let base = game_data.get();
                            base.map(|res| match res {
                        Err(e) => view! { <div class="error">"Error: " {e.to_string()}</div> }.into_any(),
                        Ok(data) => {
                            let is_turn = data.is_my_turn;
                            let is_finished = data.is_finished;
                            let id = data.id;
                            let html = data.html.clone();
                            let player_style = data.player_style.clone();
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
                                        <GameBoard html=html player_style=player_style.clone() />
                                        <RecentGameLogs player_style=player_style.clone() />
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
                }
            }}
        </MainLayout>
    }
}

/// Counts active games where it's the user's turn - the title's "(N)" badge.
/// Pure (no resource/DOM access) so it's unit-testable on its own.
fn count_my_turn(games: &[crate::game::server_fns::GameSummary]) -> usize {
    games.iter().filter(|g| g.is_turn).count()
}

/// State for GamePage's per-game WS-sequence memo: `(viewed game, last seq
/// seen for it)`. Updates for other games keep the previous seq - the old
/// closure returned None for them, which re-keyed the game resource and
/// remounted the game view (clearing the command input mid-typing).
/// Changing the viewed game resets the seq.
fn track_game_seq(
    prev: Option<(Option<Uuid>, Option<u64>)>,
    current_id: Option<Uuid>,
    update: Option<(Uuid, u64)>,
) -> (Option<Uuid>, Option<u64>) {
    let prev_seq = match prev {
        Some((prev_id, seq)) if prev_id == current_id => seq,
        _ => None,
    };
    let seq = match update {
        Some((id, seq)) if Some(id) == current_id => Some(seq),
        _ => prev_seq,
    };
    (current_id, seq)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::server_fns::{GameSummary, OpponentSummary};

    fn game_summary(is_turn: bool) -> GameSummary {
        GameSummary {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            type_name: "Test Game".to_string(),
            opponents: vec![OpponentSummary {
                name: "Bob".to_string(),
                color: "#000".to_string(),
            }],
            is_turn,
            is_turn_at: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
                time::Time::MIDNIGHT,
            ),
        }
    }

    #[test]
    fn count_my_turn_counts_only_is_turn_games() {
        let games = vec![game_summary(true), game_summary(false), game_summary(true)];
        assert_eq!(count_my_turn(&games), 2);
    }

    #[test]
    fn count_my_turn_zero_for_empty() {
        assert_eq!(count_my_turn(&[]), 0);
    }

    #[test]
    fn theme_boot_script_contains_all_theme_slugs() {
        for (slug, _) in crate::theme::THEME_SLUGS {
            assert!(
                THEME_BOOT_SCRIPT.contains(&format!("\"{slug}\"")),
                "THEME_BOOT_SCRIPT missing slug {slug}"
            );
        }
    }

    #[test]
    fn track_game_seq_retains_seq_on_other_game_updates() {
        let this_game = Uuid::new_v4();
        let other_game = Uuid::new_v4();
        // An update for this game sets the seq...
        let state = track_game_seq(None, Some(this_game), Some((this_game, 3)));
        assert_eq!(state, (Some(this_game), Some(3)));
        // ...and an update for a DIFFERENT game must keep it (the old memo
        // collapsed to None here, re-keying game_data and remounting the
        // game view mid-typing).
        let state = track_game_seq(Some(state), Some(this_game), Some((other_game, 4)));
        assert_eq!(state, (Some(this_game), Some(3)));
    }

    #[test]
    fn track_game_seq_resets_when_viewed_game_changes() {
        let game_a = Uuid::new_v4();
        let game_b = Uuid::new_v4();
        let state = track_game_seq(None, Some(game_a), Some((game_a, 7)));
        // Navigating to another game must not carry game A's seq over.
        let state = track_game_seq(Some(state), Some(game_b), Some((game_a, 7)));
        assert_eq!(state, (Some(game_b), None));
    }
}
