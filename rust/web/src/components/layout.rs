use crate::components::game::PlayerName;
use crate::game::server_fns::{GameSummary, get_active_games};
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
    view! {
        <div class="layout">
            <div class="layout-header" class:my-turn=move || is_my_turn.get()>
                <input type="button" value="Menu"/>
                <span class="header-title">"brdg.me"</span>
                // Always render same element type; toggle visibility to avoid structural mismatch
                <input type="button" value="Sub menu" hidden=move || !has_sub_menu.get()/>
                <input type="button" value="Next game" hidden=move || !has_next_game.get()/>
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

    let trigger = expect_context::<crate::websocket_client::WebSocketTrigger>();
    let active_games: LocalResource<Result<Vec<GameSummary>, ServerFnError>> =
        LocalResource::new(move || async move {
            let _ = trigger.last_update.get();
            get_active_games().await
        });

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
