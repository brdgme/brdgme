use crate::components::game::PlayerName;
use crate::game::server_fns::{GameSummary, SidebarGames};
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

/// Open state for the header "Sub menu" button's target (the game meta
/// panel). Provided by `MainLayout` so `GameMeta`, which lives deep inside
/// the page children, can toggle its `.open` class and render the close
/// underlay.
#[derive(Clone, Copy)]
pub struct SubMenuOpen {
    pub open: ReadSignal<bool>,
    pub set_open: WriteSignal<bool>,
}

/// Picks the game that has been awaiting the player's turn the longest -
/// the "Next game" button's target.
fn next_game_id(games: &[GameSummary]) -> Option<uuid::Uuid> {
    games
        .iter()
        .filter(|g| g.is_turn)
        .min_by_key(|g| g.is_turn_at)
        .map(|g| g.id)
}

#[component]
pub fn MainLayout(
    #[prop(into, default = Signal::from(false))] has_sub_menu: Signal<bool>,
    children: Children,
) -> impl IntoView {
    let (menu_open, set_menu_open) = signal(false);
    let (sub_menu_open, set_sub_menu_open) = signal(false);
    provide_context(SubMenuOpen {
        open: sub_menu_open,
        set_open: set_sub_menu_open,
    });

    // Close the sub menu overlay on every route change, mirroring the
    // sidebar menu's effect in `SidebarMenu`.
    let location = leptos_router::hooks::use_location();
    Effect::new(move |_| {
        location.pathname.get();
        set_sub_menu_open.set(false);
    });

    // Header state derives from the sidebar's active-games resource (provided
    // in `App`), not from the current page: the bar is "my turn" coloured
    // whenever ANY game is awaiting this player, on every page it is visible.
    // The resource is None during SSR/hydration so both start inactive -
    // class/attribute-only changes, no structural mismatch.
    let active_games = expect_context::<LocalResource<Result<SidebarGames, ServerFnError>>>();
    let is_my_turn = Memo::new(move |_| {
        active_games
            .get()
            .and_then(|r| r.ok())
            .is_some_and(|games| games.active.iter().any(|g| g.is_turn))
    });
    // The longest-waiting my-turn game, hidden while already viewing it.
    let next_game = Memo::new(move |_| {
        let id = active_games
            .get()
            .and_then(|r| r.ok())
            .and_then(|g| next_game_id(&g.active))?;
        (location.pathname.get() != format!("/games/{}", id)).then_some(id)
    });
    let navigate = use_navigate();

    view! {
        <div class="layout">
            <div class="layout-header" class:my-turn=move || is_my_turn.get()>
                <button
                    class="header-icon-button"
                    aria-label="Menu"
                    on:click=move |_| set_menu_open.update(|v| *v = !*v)
                >"\u{2630}"</button>
                <span class="header-title">"brdg.me"</span>
                // Always render same element type; toggle visibility to avoid structural mismatch
                <button
                    class="header-icon-button header-sub-menu"
                    aria-label="Sub menu"
                    hidden=move || !has_sub_menu.get()
                    on:click=move |_| set_sub_menu_open.update(|v| *v = !*v)
                >"\u{22ee}"</button>
                <input
                    type="button"
                    value="Next game"
                    hidden=move || next_game.get().is_none()
                    on:click=move |_| {
                        if let Some(id) = next_game.get_untracked() {
                            navigate(&format!("/games/{}", id), NavigateOptions::default());
                        }
                    }
                />
            </div>
            <div class="layout-body">
                <SidebarMenu open=menu_open set_open=set_menu_open />
                // Mobile-only overlay (see the `@media (max-width: 80em)` block in
                // main.scss); only mounted while the menu is open so it never
                // covers the page underneath it when closed.
                <Show when=move || menu_open.get()>
                    <div
                        class="menu-close-underlay"
                        on:click=move |_| set_menu_open.set(false)
                    ></div>
                </Show>
                <div class="content">
                    {children()}
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn SidebarMenu(#[prop(into)] open: Signal<bool>, set_open: WriteSignal<bool>) -> impl IntoView {
    let logout_action = expect_context::<ServerAction<crate::auth::Logout>>();
    let navigate = use_navigate();
    Effect::new(move |_| {
        if logout_action.value().get().is_some_and(|r| r.is_ok()) {
            navigate("/login", NavigateOptions::default());
        }
    });

    // Provided once in `App` (outside the router) so these resources survive
    // client-side navigation instead of being torn down and recreated by
    // every page's own `<MainLayout>` - see the comment at their
    // `provide_context` call sites in `app.rs`.
    let active_games = expect_context::<LocalResource<Result<SidebarGames, ServerFnError>>>();
    let current_user =
        expect_context::<LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>>();
    let logged_in = move || matches!(current_user.get(), Some(Ok(Some(_))));

    // Close the mobile menu overlay on every route change - covers
    // "navigating closes it" for every link without per-link handlers.
    let location = leptos_router::hooks::use_location();
    Effect::new(move |_| {
        location.pathname.get();
        set_open.set(false);
    });

    view! {
        <div class="menu" class:open=move || open.get()>
            <h1><A href="/">"brdg.me"</A></h1>
            <div class="subheading"><A href="/">"Lo-fi board games"</A></div>
            <div>
                // Same element type in both states to avoid a structural
                // hydration mismatch; LocalResource is always None during
                // SSR/hydration so both render the "Login" branch there.
                <div hidden=move || !logged_in()>
                    {move || {
                        let name = current_user
                            .get()
                            .and_then(|r| r.ok())
                            .flatten()
                            .map(|u| u.name)
                            .unwrap_or_default();
                        let href = format!("/players/{}", crate::players::encode_path_segment(&name));
                        view! {
                            <A href=href>{name}</A>
                            " ("
                            <a
                                on:click=move |_| {
                                    logout_action.dispatch(crate::auth::Logout {});
                                }
                                style="cursor:pointer"
                            >"logout"</a>
                            ")"
                        }
                    }}
                </div>
                <div hidden=logged_in>
                    <A href="/login">"Login"</A>
                </div>
            </div>
            <div><A href="/games">"New game"</A></div>
            <div><A href="/settings">"Settings"</A></div>
            <div><A href="/friends">"Friends"</A></div>
            <div>
                {move || match active_games.get() {
                    None => view! { <p>"Loading games..."</p> }.into_any(),
                    Some(Err(_)) => view! { <p class="error">"Error loading games"</p> }.into_any(),
                    Some(Ok(games)) => {
                        let mut sections: Vec<AnyView> = Vec::new();
                        if !games.active.is_empty() {
                            sections.push(view! {
                                <h2>"Active games"</h2>
                                {games.active.into_iter().map(|game| {
                                    let id = game.id.to_string();
                                    view! {
                                        <div class="layout-game" class:my-turn=game.is_turn>
                                            <A href=format!("/games/{}", id)>
                                                <div class="layout-game-name">{game.type_name}</div>
                                                <div class="layout-game-opponents">
                                                    "with "
                                                    {game.opponents.into_iter().map(|opp| view! {
                                                        <span>" " <PlayerName name=opp.name color=opp.color /></span>
                                                    }).collect_view()}
                                                </div>
                                            </A>
                                        </div>
                                    }.into_any()
                                }).collect_view()}
                            }.into_any());
                        }
                        if !games.pending.is_empty() {
                            sections.push(view! {
                                <h2>"Pending games"</h2>
                                {games.pending.into_iter().map(|game| {
                                    let id = game.id.to_string();
                                    let needs_action = if game.is_owner {
                                        game.is_ready_to_start
                                    } else {
                                        game.is_invitee_needing_accept
                                    };
                                    let name = if game.is_invitee_needing_accept {
                                        format!("{} (Invite)", game.type_name)
                                    } else {
                                        game.type_name.clone()
                                    };
                                    view! {
                                        <div class="layout-game" class:my-turn=needs_action>
                                            <A href=format!("/invites/{}", id)>
                                                <div class="layout-game-name">{name}</div>
                                                <div class="layout-game-opponents">
                                                    "with "
                                                    {game.players.into_iter().map(|p| view! {
                                                        <span>" " {p}</span>
                                                    }).collect_view()}
                                                </div>
                                            </A>
                                        </div>
                                    }.into_any()
                                }).collect_view()}
                            }.into_any());
                        }
                        if !games.finished.is_empty() {
                            sections.push(view! {
                                <h2>"Finished games"</h2>
                                {games.finished.into_iter().map(|game| {
                                    let id = game.id.to_string();
                                    view! {
                                        <div class="layout-game">
                                            <A href=format!("/games/{}", id)>
                                                <div class="layout-game-name">{game.type_name}</div>
                                                <div class="layout-game-opponents">
                                                    "with "
                                                    {game.players.into_iter().map(|p| view! {
                                                        <span>" " {p}</span>
                                                    }).collect_view()}
                                                </div>
                                            </A>
                                        </div>
                                    }.into_any()
                                }).collect_view()}
                            }.into_any());
                        }
                        sections.collect_view().into_any()
                    },
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::server_fns::GameSummary;
    use uuid::Uuid;

    fn game_summary(is_turn: bool, hour: u8) -> GameSummary {
        GameSummary {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            type_name: "Test Game".to_string(),
            opponents: Vec::new(),
            is_turn,
            is_turn_at: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
                time::Time::from_hms(hour, 0, 0).unwrap(),
            ),
        }
    }

    #[test]
    fn next_game_id_picks_longest_waiting_my_turn_game() {
        let games = vec![
            game_summary(false, 1),
            game_summary(true, 9),
            game_summary(true, 3),
        ];
        assert_eq!(next_game_id(&games), Some(games[2].id));
    }

    #[test]
    fn next_game_id_none_when_no_game_is_my_turn() {
        let games = vec![game_summary(false, 1)];
        assert_eq!(next_game_id(&games), None);
    }
}
