use crate::components::game::PlayerName;
use crate::game::server_fns::GameSummary;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

#[component]
pub fn MainLayout(
    #[prop(into, default = Signal::from(false))] is_my_turn: Signal<bool>,
    #[prop(into, default = Signal::from(false))] has_sub_menu: Signal<bool>,
    #[prop(into, default = Signal::from(false))] has_next_game: Signal<bool>,
    children: Children,
) -> impl IntoView {
    let (menu_open, set_menu_open) = signal(false);

    view! {
        <div class="layout">
            <div class="layout-header" class:my-turn=move || is_my_turn.get()>
                <input
                    type="button"
                    value="Menu"
                    on:click=move |_| set_menu_open.update(|v| *v = !*v)
                />
                <span class="header-title">"brdg.me"</span>
                // Always render same element type; toggle visibility to avoid structural mismatch
                <input type="button" value="Sub menu" hidden=move || !has_sub_menu.get()/>
                <input type="button" value="Next game" hidden=move || !has_next_game.get()/>
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
    let on_logout = move |_| {
        logout_action.dispatch(crate::auth::Logout {});
    };

    // Provided once in `App` (outside the router) so these resources survive
    // client-side navigation instead of being torn down and recreated by
    // every page's own `<MainLayout>` - see the comment at their
    // `provide_context` call sites in `app.rs`.
    let active_games = expect_context::<LocalResource<Result<Vec<GameSummary>, ServerFnError>>>();
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
                    <a on:click=on_logout style="cursor:pointer">"Logout"</a>
                </div>
                <div hidden=logged_in>
                    <A href="/login">"Login"</A>
                </div>
            </div>
            <div><A href="/games">"New game"</A></div>
            <div>
                <h2>"Active games"</h2>
                {move || match active_games.get() {
                    None => view! { <p>"Loading games..."</p> }.into_any(),
                    Some(Err(_)) => view! { <p class="error">"Error loading games"</p> }.into_any(),
                    Some(Ok(games)) => {
                        if games.is_empty() {
                            view! { <p class="no-games">"No active games"</p> }.into_any()
                        } else {
                            games.into_iter().map(|game| {
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
                            }).collect_view().into_any()
                        }
                    },
                }}
            </div>
        </div>
    }
}
