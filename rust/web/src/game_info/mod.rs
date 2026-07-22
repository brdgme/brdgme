use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "ssr")]
use crate::error::internal;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameInfoRankingEntry {
    pub user_id: Uuid,
    pub name: String,
    pub rating: i32,
    pub peak_rating: i32,
    pub form: Vec<crate::stats::FormResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameInfoData {
    pub name: String,
    pub blurb: String,
    pub rules_version_id: Option<Uuid>,
    pub total_games: i64,
    pub active_today: i64,
    pub distinct_players: i64,
    pub ranking: Vec<GameInfoRankingEntry>,
}

#[cfg(feature = "ssr")]
mod queries;

#[cfg(feature = "ssr")]
pub use queries::*;

#[server(GetGameInfo, "/api")]
pub async fn get_game_info(name: String) -> Result<Option<GameInfoData>, ServerFnError> {
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();

    let (game_type_id, name, blurb) = match queries::game_info_header(&pool, &name)
        .await
        .map_err(internal("get_game_info: header"))?
    {
        Some(header) => header,
        None => return Ok(None),
    };

    let rules_version_id = queries::game_info_rules_version_id(&pool, game_type_id)
        .await
        .map_err(internal("get_game_info: rules_version_id"))?;
    let total_games = queries::game_info_total_games(&pool, game_type_id)
        .await
        .map_err(internal("get_game_info: total_games"))?;
    let active_today = queries::game_info_active_today(&pool, game_type_id)
        .await
        .map_err(internal("get_game_info: active_today"))?;
    let distinct_players = queries::game_info_distinct_players(&pool, game_type_id)
        .await
        .map_err(internal("get_game_info: distinct_players"))?;
    let ranking_rows = queries::game_info_top_ranking(&pool, game_type_id)
        .await
        .map_err(internal("get_game_info: top_ranking"))?;

    let user_ids: Vec<Uuid> = ranking_rows.iter().map(|(id, _, _, _)| *id).collect();
    let form = crate::stats::recent_form_for_game_type(&pool, &user_ids, game_type_id, 10)
        .await
        .map_err(internal("get_game_info: form"))?;

    let ranking = ranking_rows
        .into_iter()
        .map(
            |(user_id, name, rating, peak_rating)| GameInfoRankingEntry {
                form: form.get(&user_id).cloned().unwrap_or_default(),
                user_id,
                name,
                rating,
                peak_rating,
            },
        )
        .collect();

    Ok(Some(GameInfoData {
        name,
        blurb,
        rules_version_id,
        total_games,
        active_today,
        distinct_players,
        ranking,
    }))
}

#[component]
pub fn GameInfoPage() -> impl IntoView {
    use crate::components::MainLayout;
    use crate::players::{encode_path_segment, rating_trend};
    use crate::stats::viz::Sparkline;
    use leptos_router::components::A;

    let params = leptos_router::hooks::use_params_map();
    let data_res = Resource::new_blocking(
        move || params.get().get("name").unwrap_or_default(),
        |name| async move { crate::game_info::get_game_info(name).await },
    );

    view! {
        <MainLayout>
            <Suspense fallback=|| view! { <div></div> }>
                {move || {
                    let data = data_res.get();
                    data.map(|res| match res {
                        Err(e) => view! {
                            <div class="error">"Error: " {e.to_string()}</div>
                        }.into_any(),
                        Ok(None) => view! {
                            <div class="game-info content-page">
                                <h1>"Game not found"</h1>
                                <p>"No such game type."</p>
                            </div>
                        }.into_any(),
                        Ok(Some(d)) => {
                            let start_href = format!(
                                "/games/new/{}",
                                encode_path_segment(&d.name),
                            );
                            let ranking = d.ranking.clone();
                            let ranking_view = if ranking.is_empty() {
                                view! { <p>"No rated players yet."</p> }.into_any()
                            } else {
                                let rows = ranking
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, entry)| {
                                        let rank = i + 1;
                                        let href = format!(
                                            "/players/{}",
                                            encode_path_segment(&entry.name),
                                        );
                                        let trend = rating_trend(
                                            Some(entry.rating),
                                            &entry.form,
                                        );
                                        let trend_view = if trend.len() >= 2 {
                                            view! { <Sparkline values=trend/> }.into_any()
                                        } else {
                                            view! { "-" }.into_any()
                                        };
                                        view! {
                                            <tr>
                                                <td>{rank}</td>
                                                <td><A href=href>{entry.name.clone()}</A></td>
                                                <td>{entry.rating}</td>
                                                <td>{entry.peak_rating}</td>
                                                <td>{trend_view}</td>
                                            </tr>
                                        }
                                    })
                                    .collect_view();
                                view! {
                                    <div class="table-scroll">
                                        <table>
                                            <thead>
                                                <tr>
                                                    <th>"Rank"</th>
                                                    <th>"Player"</th>
                                                    <th>"Rating"</th>
                                                    <th>"Peak"</th>
                                                    <th>"Trend"</th>
                                                </tr>
                                            </thead>
                                            <tbody>{rows}</tbody>
                                        </table>
                                    </div>
                                }.into_any()
                            };
                            view! {
                                <div class="game-info content-page">
                                    <h1>{d.name.clone()}</h1>
                                    {(!d.blurb.is_empty())
                                        .then(|| view! { <p class="game-info-blurb">{d.blurb.clone()}</p> })}
                                    {d.rules_version_id.map(|vid| view! {
                                        <p><A href=format!("/rules/{}", vid)>"Rules & strategy"</A></p>
                                    })}
                                    <section class="game-info-stats">
                                        <p>"Games played: " {d.total_games}</p>
                                        <p>"Active today: " {d.active_today}</p>
                                        <p>"Players: " {d.distinct_players}</p>
                                    </section>
                                    <p class="game-info-start">
                                        <A href=start_href>"Start a game"</A>
                                    </p>
                                    <section class="game-info-ranking">
                                        <h2>"Top players"</h2>
                                        {ranking_view}
                                    </section>
                                </div>
                            }.into_any()
                        }
                    })
                }}
            </Suspense>
        </MainLayout>
    }
}
