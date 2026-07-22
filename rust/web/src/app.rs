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

/// Presence ping cadence: while a page is open the client tells the server
/// it's active this often. The server's recency window is 2x this (see
/// `db::RECENTLY_ACTIVE_WINDOW`).
const PRESENCE_PING_INTERVAL_MS: u32 = 5 * 60 * 1000;

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
    format!(
        r#"window.Sentry.init({{"dsn":"{}","integrations":[window.SentryWasmIntegration(),window.Sentry.browserTracingIntegration()],"sendDefaultPii":false,"tracesSampleRate":0.1{},"beforeSend":{}}});"#,
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
    let proposal_update = RwSignal::<Option<(Uuid, u64)>>::new(None);
    provide_context(crate::websocket_client::ProposalUpdate(proposal_update));
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

    let active_games: LocalResource<Result<crate::game::server_fns::SidebarGames, ServerFnError>> =
        LocalResource::new(move || async move {
            let _ = last_update.get();
            crate::game::server_fns::get_sidebar_games().await
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

    // Presence ping: while logged in with any page open, tell the server we're
    // active every 5 min. No Page Visibility gating - an open page counts.
    // Runs only on hydrate (Effects are inert during SSR). The loop breaks once
    // the user is no longer logged in, so it can't outlive the session.
    let presence_started = RwSignal::new(false);
    Effect::new(move |_| {
        if matches!(current_user.get(), Some(Ok(Some(_)))) && !presence_started.get_untracked() {
            presence_started.set(true);
            leptos::task::spawn_local(async move {
                loop {
                    if !matches!(current_user.get_untracked(), Some(Ok(Some(_)))) {
                        break;
                    }
                    let _ = crate::auth::ping_active().await;
                    gloo_timers::future::TimeoutFuture::new(PRESENCE_PING_INTERVAL_MS).await;
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
            .map(|games| count_my_turn(&games.active))
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
                // "/games" is reserved (currently unused); the new-game flow lives at "/games/new".
                // <Route path=StaticSegment("games") view=crate::new_game::NewGamePage/>
                <Route path=StaticSegment("settings") view=crate::settings::SettingsPage/>
                <Route path=StaticSegment("friends") view=crate::friends::FriendsPage/>
                <Route path=StaticSegment("admin") view=crate::admin::AdminPage/>
                <Route path=(StaticSegment("players"), ParamSegment("name")) view=crate::players::PlayersPage/>
                <Route path=(StaticSegment("players"), ParamSegment("name"), StaticSegment("history")) view=crate::players::PlayerHistoryPage/>
                <Route path=(StaticSegment("players"), ParamSegment("name"), ParamSegment("game_type")) view=crate::players::PlayerGameTypePage/>
                <Route path=(StaticSegment("games"), StaticSegment("type"), ParamSegment("name")) view=crate::game_info::GameInfoPage/>
                <Route path=(StaticSegment("games"), StaticSegment("new")) view=crate::new_game::NewGameTypePage/>
                <Route path=(StaticSegment("games"), StaticSegment("new"), ParamSegment("type")) view=crate::new_game::NewGameSetupPage/>
                <Route path=(StaticSegment("games"), ParamSegment("id")) view=GamePage/>
                <Route path=(StaticSegment("rules"), ParamSegment("version_id")) view=crate::rules::RulesPage/>
                <Route path=(StaticSegment("invites"), ParamSegment("id")) view=crate::proposals::InvitePage/>
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
    use crate::components::game::GameBoard;
    use crate::stats::viz::Sparkline;

    let current_user =
        expect_context::<LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>>();
    let logged_in = move || matches!(current_user.get(), Some(Ok(Some(_))));

    let trigger = expect_context::<crate::websocket_client::WebSocketTrigger>();
    let public_index: LocalResource<
        Result<Option<crate::game::server_fns::PublicIndexGame>, ServerFnError>,
    > = LocalResource::new(move || async move {
        let _ = trigger.last_update.get();
        crate::game::server_fns::get_public_index().await
    });

    let index_data: LocalResource<Result<crate::index::LoggedInIndexData, ServerFnError>> =
        LocalResource::new(crate::index::get_logged_in_index);

    let mounted = RwSignal::new(false);
    Effect::new(move |_| mounted.set(true));

    let title = move || {
        public_index
            .get()
            .and_then(|r| r.ok())
            .flatten()
            .map(|g| g.type_name)
            .unwrap_or_else(|| "brdg.me".to_string())
    };

    let has_game = move || mounted.get() && matches!(public_index.get(), Some(Ok(Some(_))));

    view! {
        <MainLayout>
            <div class="content-page index-logged-out" hidden=logged_in>
                <h1>{title}</h1>
                <p class="index-subheading">"Lo-fi board games by email and web"</p>
                <A href="/login" attr:class="index-cta">"Start a game"</A>
                <div class="index-game-section" hidden=move || !has_game()>
                    {move || {
                        public_index.get().and_then(|r| r.ok()).flatten().map(|game| {
                            let logs: Vec<_> = game.logs.iter().take(3).cloned().collect();
                            view! {
                                <GameBoard html=game.html player_style=game.player_style />
                                <div class="index-logs">
                                    {logs.into_iter().map(|entry| {
                                        view! { <div class="game-log-entry" inner_html=entry.body_html></div> }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }
                        })
                    }}
                </div>
            </div>
            <div class="content-page index-logged-in" hidden=move || !logged_in()>
                <section class="index-friends">
                    <h2>"Friends"</h2>
                    {move || match index_data.get() {
                        None => view! { <p>"Loading..."</p> }.into_any(),
                        Some(Err(_)) => ().into_any(),
                        Some(Ok(d)) if d.friends.is_empty() =>
                            view! { <p>"No friends yet."</p> }.into_any(),
                        Some(Ok(d)) => d.friends.iter().map(|f| {
                            let name = f.friend_name.clone();
                            let href = format!("/players/{}", crate::players::encode_path_segment(&name));
                            let recent = match (f.game_id, f.game_type_name.clone()) {
                                (Some(gid), Some(gtype)) => {
                                    let game_href = format!("/games/{}", gid);
                                    view! {
                                        <span class="index-friend-recent">
                                            " - " <A href=game_href>{gtype}</A>
                                        </span>
                                    }.into_any()
                                }
                                _ => ().into_any(),
                            };
                            view! {
                                <div class="index-friend">
                                    <A href=href>{name}</A>
                                    {recent}
                                </div>
                            }
                        }).collect_view().into_any(),
                    }}
                </section>

                <section class="index-game-types">
                    <h2>"Ratings"</h2>
                    {move || match index_data.get() {
                        None => view! { <p>"Loading..."</p> }.into_any(),
                        Some(Err(_)) => ().into_any(),
                        Some(Ok(d)) if d.game_types.is_empty() =>
                            view! { <p>"No games played yet."</p> }.into_any(),
                        Some(Ok(d)) => {
                            let rows = d.game_types.iter().map(|gt| {
                                let href = format!("/games/type/{}", crate::players::encode_path_segment(&gt.game_type_name));
                                let rating = gt.rating.map(|r| r.to_string()).unwrap_or_else(|| "-".to_string());
                                let trend = gt.trend.clone();
                                let name = gt.game_type_name.clone();
                                view! {
                                    <tr>
                                        <td><A href=href>{name}</A></td>
                                        <td>{rating}</td>
                                        <td>
                                            {if trend.len() >= 2 {
                                                view! { <Sparkline values=trend/> }.into_any()
                                            } else {
                                                view! { "-" }.into_any()
                                            }}
                                        </td>
                                    </tr>
                                }
                            }).collect_view();
                            view! {
                                <div class="table-scroll">
                                    <table>
                                        <thead>
                                            <tr><th>"Game"</th><th>"Rating"</th><th>"Trend"</th></tr>
                                        </thead>
                                        <tbody>{rows}</tbody>
                                    </table>
                                </div>
                            }.into_any()
                        }
                    }}
                </section>

                <section class="index-history">
                    <h2>"Recent games"</h2>
                    {move || match index_data.get() {
                        None => view! { <p>"Loading..."</p> }.into_any(),
                        Some(Err(_)) => ().into_any(),
                        Some(Ok(d)) if d.history.is_empty() =>
                            view! { <p>"No games yet."</p> }.into_any(),
                        Some(Ok(d)) => {
                            let rows = d.history.iter().map(|h| {
                                let href = format!("/games/{}", h.game_id);
                                let my_turn = !h.is_finished && h.is_turn;
                                let finished = h.is_finished;
                                let status = if h.is_finished {
                                    "Finished"
                                } else if h.is_turn {
                                    "Your turn"
                                } else {
                                    "Active"
                                };
                                let name = h.game_type_name.clone();
                                view! {
                                    <tr class:my-turn=my_turn class:finished=finished>
                                        <td><A href=href>{name}</A></td>
                                        <td>{status}</td>
                                    </tr>
                                }
                            }).collect_view();
                            view! {
                                <div class="table-scroll">
                                    <table>
                                        <thead>
                                            <tr><th>"Game"</th><th>"Status"</th></tr>
                                        </thead>
                                        <tbody>{rows}</tbody>
                                    </table>
                                </div>
                            }.into_any()
                        }
                    }}
                </section>
            </div>
        </MainLayout>
    }
}

#[component]
fn LoginPage() -> impl IntoView {
    let current_user =
        expect_context::<LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>>();
    let active_games = expect_context::<
        LocalResource<Result<crate::game::server_fns::SidebarGames, ServerFnError>>,
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

    // Navigate to the index on successful login. `current_user`/`active_games`
    // live above the Router (see App()), so this navigation no longer
    // remounts them - refetch explicitly or the sidebar keeps showing
    // logged-out state and no active games until a hard reload.
    let navigate = use_navigate();
    Effect::new(move |_| {
        if confirm_action.value().get().is_some_and(|r| r.is_ok()) {
            current_user.refetch();
            active_games.refetch();
            navigate("/", NavigateOptions::default());
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
                        // game_data is stale-while-revalidate: during a game-to-game
                        // navigation refetch, .get() still returns the previous game's
                        // data. Show the loading spinner instead of the stale board.
                        Ok(data) if current_game.get() != Some(data.id) => {
                            view! { <div class="game-loading"><crate::components::Spinner/></div> }.into_any()
                        }
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
                                    .map(|p| (p.name.clone(), p.color.clone(), p.is_bot))
                                    .collect::<Vec<_>>()
                            );
                            view! {
                                <div class="game-container">
                                    <div class="game-main">
                                        <GameBoard html=html player_style=player_style.clone() />
                                        // logs is a LocalResource that never resolves on SSR: this Suspense
                                        // keeps the outer Transition from emitting fallback HTML on the
                                        // server, and the mounted-gate inside RecentGameLogs keeps SSR and
                                        // hydration output identical (see the comment in GameLogs).
                                        <Suspense fallback=|| ()>
                                            <RecentGameLogs player_style=player_style.clone() />
                                        </Suspense>
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
                                                {waiting_on.with_value(|w| w.iter().enumerate().map(|(i, (name, color, is_bot))| {
                                                    let name = name.clone();
                                                    let color = color.clone();
                                                    let is_bot = *is_bot;
                                                    view! {
                                                        <span>
                                                            {if i > 0 { ", " } else { "" }}
                                                            <PlayerName name=name color=color profile_link=!is_bot />
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
