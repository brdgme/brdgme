// Layout components for brdg.me

use leptos::prelude::*;
use leptos_router::components::A;

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
    view! {
        <div class="menu">
            <h1><a>"brdg.me"</a></h1>
            <div class="subheading"><a>"Lo-fi board games"</a></div>
            <div>
                <div><a>"Logout"</a></div>
            </div>
            <div><a>"New game"</a></div>
            <div>
                <h2>"Active games"</h2>
                <div class="layout-game">
                    <A href="/games/1">
                        <div class="layout-game-name">"Sushizock im Gockelwok"</div>
                        <div class="layout-game-opponents">"with " <span>" " <strong class="brdgme-purple">"<Elly>"</strong></span><span>" " <strong class="brdgme-red">"<baconheist>"</strong></span></div>
                    </A>
                </div>
                <div class="layout-game">
                    <A href="/games/2">
                        <div class="layout-game-name">"Lost Cities"</div>
                        <div class="layout-game-opponents">"with " <span>" " <strong class="brdgme-red">"<baconheist>"</strong></span></div>
                    </A>
                </div>
                <div class="layout-game">
                    <A href="/games/3">
                        <div class="layout-game-name">"Acquire"</div>
                        <div class="layout-game-opponents">"with " <span>" " <strong class="brdgme-red">"<baconheist>"</strong></span></div>
                    </A>
                </div>
            </div>
        </div>
    }
}