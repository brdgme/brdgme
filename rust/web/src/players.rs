//! #29 player stats: the /players/:name profile page.

use leptos::prelude::*;

#[component]
pub fn PlayersPage() -> impl IntoView {
    use crate::components::layout::MainLayout;

    let params = leptos_router::hooks::use_params_map();
    let query = leptos_router::hooks::use_query_map();
    let profile = Resource::new_blocking(
        move || {
            (
                params.get().get("name").unwrap_or_default(),
                query.get().get("bots").as_deref() == Some("1"),
            )
        },
        |(name, include_single_human)| async move {
            crate::stats::get_player_profile(name, include_single_human).await
        },
    );

    view! {
        <MainLayout>
            <Suspense fallback=|| view! { <div></div> }>
                {move || {
                    let data = profile.get();
                    data.map(|res| match res {
                        Err(e) => view! {
                            <div class="error">"Error: " {e.to_string()}</div>
                        }.into_any(),
                        Ok(None) => view! {
                            <div class="profile content-page">
                                <h1>"Player not found"</h1>
                                <p>"No such player."</p>
                            </div>
                        }.into_any(),
                        Ok(Some(d)) => {
                            let win_rate = if d.totals.finished_games == 0 {
                                "-".to_string()
                            } else {
                                format!("{:.1}%", d.totals.win_percent)
                            };
                            let name_style = d
                                .user
                                .pref_color
                                .as_ref()
                                .map(|c| format!("color:var(--mk-{})", c.to_lowercase()));
                            view! {
                                <div class="profile content-page">
                                    <header class="profile-header">
                                        <h1><span style=name_style>{d.user.name.clone()}</span></h1>
                                        <p class="profile-member-since">
                                            "Member since " {d.user.created_at.date().to_string()}
                                        </p>
                                    </header>
                                    <section class="profile-overall-stats">
                                        <h2>"Overall"</h2>
                                        <p>"Finished games: " {d.totals.finished_games}</p>
                                        <p>"Wins: " {d.totals.wins}</p>
                                        <p>"Win rate: " {win_rate}</p>
                                    </section>
                                    // TODO(#29): per-game-type stats table - owned by a later unit.
                                    <section class="profile-game-types"></section>
                                    // TODO(#29): recent form strips + recent finished games list - later unit.
                                    <section class="profile-recent-games"></section>
                                    // TODO(#29): active games list (own profile only) - later unit.
                                    <section class="profile-active-games"></section>
                                </div>
                            }.into_any()
                        }
                    })
                }}
            </Suspense>
        </MainLayout>
    }
}
