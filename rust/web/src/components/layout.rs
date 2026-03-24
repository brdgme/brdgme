use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use crate::components::game::PlayerName;

use crate::game::server_fns::GameSummary;

#[component]
pub fn MainLayout(
    #[prop(default = false)] is_my_turn: bool,
    #[prop(default = false)] has_sub_menu: bool,
    #[prop(default = false)] has_next_game: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="layout">
            <div class="layout-header" class:my-turn=is_my_turn>
                <input type="button" value="Menu"/>
                <span class="header-title">"brdg.me"</span>
                {if has_sub_menu {
                    view! { <input type="button" value="Sub menu"/> }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
                {if has_next_game {
                    view! { <input type="button" value="Next game"/> }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
            </div>
            <div class="layout-body">
                <SidebarMenu />
                <div class="content">
                    {children()}
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn SidebarMenu() -> impl IntoView {
    let logout_action = ServerAction::<crate::auth::Logout>::new();
    let navigate = use_navigate();
    Effect::new(move |_| {
        if logout_action.value().get().is_some_and(|r| r.is_ok()) {
            navigate("/login", NavigateOptions::default());
        }
    });
    let on_logout = move |_| {
        logout_action.dispatch(crate::auth::Logout {});
    };

    let active_games = expect_context::<Resource<Result<Vec<GameSummary>, ServerFnError>>>();

    view! {
        <div class="menu">
            <h1><A href="/">"brdg.me"</A></h1>
            <div class="subheading"><A href="/">"Lo-fi board games"</A></div>
            <div>
                <div><a on:click=on_logout style="cursor:pointer">"Logout"</a></div>
            </div>
            <div><A href="/games">"New game"</A></div>
            <div>
                <h2>"Active games"</h2>
                <Suspense fallback=move || view! { <p>"Loading games..."</p> }>
                    {move || {
                        active_games.get().map(|res| match res {
                            Ok(games) => {
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
                            Err(_) => view! { <p class="error">"Error loading games"</p> }.into_any(),
                        })
                    }}
                </Suspense>
            </div>
        </div>
    }
}