//! #29 player stats: the /players/:name profile page.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::stats::FinishedGameRow;
use crate::stats::FormResult;
use crate::stats::viz::{FormStrip, Histogram, HistogramBucket, RatingChart, Sparkline};

/// Reconstructs the recent rating series for one game type by walking
/// backward from the current rating over the rated games' rating changes.
/// The last rated game in the window necessarily produced the current
/// rating, so the series ends exactly at `current`.
fn rating_trend(current: Option<i32>, results: &[FormResult]) -> Vec<f64> {
    let changes: Vec<i32> = results.iter().filter_map(|r| r.rating_change).collect();
    let Some(current) = current else {
        return Vec::new();
    };
    if changes.len() < 2 {
        return Vec::new();
    }
    let mut r = current;
    let mut series = Vec::with_capacity(changes.len());
    for c in changes.iter().rev() {
        series.push(r as f64);
        r -= c;
    }
    series.reverse();
    series
}

/// Percent-encodes a string for use as a single URL path segment.
fn encode_path_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// English ordinal suffix for a 1-based placing (1st, 2nd, 3rd, 4th, 11th..13th).
fn ordinal_suffix(n: i32) -> &'static str {
    if (11..=13).contains(&n.rem_euclid(100)) {
        return "th";
    }
    match n.rem_euclid(10) {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    }
}

/// Formats a finished-game placing as e.g. "1st of 4"; `None` place is "-".
fn format_placing(place: Option<i32>, player_count: i64) -> String {
    match place {
        None => "-".to_string(),
        Some(p) => format!("{}{} of {}", p, ordinal_suffix(p), player_count),
    }
}

/// Renders a comma-separated opponent list; human opponents link to their
/// profile, bots (no user_id) render as plain text. Never nests an <A> inside
/// another <A> - each opponent gets its own inline span.
fn opponents_view(opponents: Vec<crate::stats::Opponent>) -> impl IntoView {
    let items = opponents
        .into_iter()
        .enumerate()
        .map(|(i, o)| {
            let prefix = if i > 0 { ", " } else { "" };
            let name = match o.user_id {
                Some(_) => {
                    let href = format!("/players/{}", encode_path_segment(&o.name));
                    view! { <A href=href>{o.name.clone()}</A> }.into_any()
                }
                None => view! { {o.name.clone()} }.into_any(),
            };
            view! { <span>{prefix}{name}</span> }
        })
        .collect_view();
    view! { <span>{items}</span> }
}

/// A placing-distribution histogram for one player-count bucket ("2
/// players" / "3 players" / "4+ players").
#[derive(Debug, Clone, PartialEq)]
pub struct PlacingHistogram {
    pub label: String,
    pub buckets: Vec<HistogramBucket>,
    pub games: i64,
    pub wins: i64,
}

/// Buckets finished games by player count (2 / 3 / 4+) and, within each
/// bucket, counts occurrences of each placing. Rows with `place: None` are
/// excluded from both the histogram bars and the games/wins totals. Buckets
/// with no placed games are omitted entirely.
fn placing_histograms(games: &[FinishedGameRow]) -> Vec<PlacingHistogram> {
    let labels = ["2 players", "3 players", "4+ players"];
    let mut grouped: [Vec<i32>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for g in games {
        let Some(place) = g.place else { continue };
        let idx = match g.player_count {
            2 => 0,
            3 => 1,
            n if n >= 4 => 2,
            _ => continue,
        };
        grouped[idx].push(place);
    }

    grouped
        .into_iter()
        .zip(labels.iter())
        .filter_map(|(placed, label)| {
            if placed.is_empty() {
                return None;
            }
            let max_place = placed.iter().cloned().max().unwrap_or(0);
            let buckets = (1..=max_place)
                .map(|p| HistogramBucket {
                    label: format!("{}{}", p, ordinal_suffix(p)),
                    count: placed.iter().filter(|&&x| x == p).count() as i64,
                })
                .collect();
            let games = placed.len() as i64;
            let wins = placed.iter().filter(|&&x| x == 1).count() as i64;
            Some(PlacingHistogram {
                label: label.to_string(),
                buckets,
                games,
                wins,
            })
        })
        .collect()
}

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
                                    <div class="profile-bots-toggle">
                                        {
                                            let toggle_name = d.user.name.clone();
                                            move || {
                                                if query.get().get("bots").as_deref() == Some("1") {
                                                    let href = format!(
                                                        "/players/{}",
                                                        encode_path_segment(&toggle_name),
                                                    );
                                                    view! {
                                                        <A href=href>
                                                            "Showing bot-only games - exclude them"
                                                        </A>
                                                    }.into_any()
                                                } else {
                                                    let href = format!(
                                                        "/players/{}?bots=1",
                                                        encode_path_segment(&toggle_name),
                                                    );
                                                    view! {
                                                        <A href=href>"Include bot-only games"</A>
                                                    }.into_any()
                                                }
                                            }
                                        }
                                    </div>
                                    <section class="profile-overall-stats">
                                        <h2>"Overall"</h2>
                                        <p>"Finished games: " {d.totals.finished_games}</p>
                                        <p>"Wins: " {d.totals.wins}</p>
                                        <p>"Win rate: " {win_rate}</p>
                                    </section>
                                    <section class="profile-game-types">
                                        <h2>"By game type"</h2>
                                        {if d.game_types.is_empty() {
                                            view! {
                                                <p class="profile-no-games">"No finished games yet."</p>
                                            }.into_any()
                                        } else {
                                            let bots = query.get().get("bots").as_deref() == Some("1");
                                            let player_name = d.user.name.clone();
                                            let game_types = d.game_types.clone();
                                            let recent_form = d.recent_form.clone();
                                            let rows = game_types.into_iter().map(|s| {
                                                let form = recent_form
                                                    .iter()
                                                    .find(|f| f.game_type_name == s.game_type_name);
                                                let results = form.map(|f| f.results.clone()).unwrap_or_default();
                                                let trend = rating_trend(s.rating, &results);
                                                let win_percent = if s.games == 0 {
                                                    "-".to_string()
                                                } else {
                                                    format!("{:.1}%", s.win_percent)
                                                };
                                                let avg_place = s
                                                    .avg_place_percentile
                                                    .map(|p| format!("{:.0}%", p * 100.0))
                                                    .unwrap_or_else(|| "-".to_string());
                                                let rating = s
                                                    .rating
                                                    .map(|r| r.to_string())
                                                    .unwrap_or_else(|| "-".to_string());
                                                let peak_rating = s
                                                    .peak_rating
                                                    .map(|r| r.to_string())
                                                    .unwrap_or_else(|| "-".to_string());
                                                let mut href = format!(
                                                    "/players/{}/{}",
                                                    encode_path_segment(&player_name),
                                                    encode_path_segment(&s.game_type_name),
                                                );
                                                if bots {
                                                    href.push_str("?bots=1");
                                                }
                                                view! {
                                                    <tr>
                                                        <td><A href=href>{s.game_type_name.clone()}</A></td>
                                                        <td>{s.games}</td>
                                                        <td>{s.wins}</td>
                                                        <td>{win_percent}</td>
                                                        <td>{avg_place}</td>
                                                        <td>{rating}</td>
                                                        <td>{peak_rating}</td>
                                                        <td>
                                                            {if trend.len() >= 2 {
                                                                view! { <Sparkline values=trend/> }.into_any()
                                                            } else {
                                                                view! { "-" }.into_any()
                                                            }}
                                                        </td>
                                                        <td>
                                                            {if results.is_empty() {
                                                                view! { "-" }.into_any()
                                                            } else {
                                                                view! { <FormStrip results=results.clone()/> }.into_any()
                                                            }}
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect_view();
                                            view! {
                                                <div class="table-scroll">
                                                    <table>
                                                        <thead>
                                                            <tr>
                                                                <th>"Game"</th>
                                                                <th>"Games"</th>
                                                                <th>"Wins"</th>
                                                                <th>"Win %"</th>
                                                                <th>"Avg placing"</th>
                                                                <th>"Rating"</th>
                                                                <th>"Peak"</th>
                                                                <th>"Trend"</th>
                                                                <th>"Form"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody>{rows}</tbody>
                                                    </table>
                                                </div>
                                            }.into_any()
                                        }}
                                    </section>
                                    <section class="profile-recent-games">
                                        <h2>"Recent games"</h2>
                                        {if d.recent_finished.is_empty() {
                                            view! {
                                                <p class="profile-no-games">"No finished games yet."</p>
                                            }.into_any()
                                        } else {
                                            let recent_finished = d.recent_finished.clone();
                                            let rows = recent_finished.into_iter().map(|row| {
                                                let href = format!("/games/{}", row.game_id);
                                                let finished = row
                                                    .finished_at
                                                    .map(|t| t.date().to_string())
                                                    .unwrap_or_else(|| "-".to_string());
                                                let placing = format_placing(row.place, row.player_count);
                                                let rating = match row.rating_change {
                                                    None => view! { "-" }.into_any(),
                                                    Some(n) if n > 0 => view! {
                                                        <span class="rating-change-up">{format!("+{n}")}</span>
                                                    }.into_any(),
                                                    Some(n) if n < 0 => view! {
                                                        <span class="rating-change-down">{n.to_string()}</span>
                                                    }.into_any(),
                                                    Some(_) => view! {
                                                        <span class="rating-change-none">"0"</span>
                                                    }.into_any(),
                                                };
                                                let opponents = opponents_view(row.opponents);
                                                view! {
                                                    <tr>
                                                        <td><A href=href>{row.game_type_name.clone()}</A></td>
                                                        <td>{finished}</td>
                                                        <td>{placing}</td>
                                                        <td>{rating}</td>
                                                        <td>{opponents}</td>
                                                    </tr>
                                                }
                                            }).collect_view();
                                            view! {
                                                <div class="table-scroll">
                                                    <table>
                                                        <thead>
                                                            <tr>
                                                                <th>"Game"</th>
                                                                <th>"Finished"</th>
                                                                <th>"Placing"</th>
                                                                <th>"Rating"</th>
                                                                <th>"Opponents"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody>{rows}</tbody>
                                                    </table>
                                                </div>
                                            }.into_any()
                                        }}
                                    </section>
                                    <section class="profile-active-games">
                                        <h2>"Active games"</h2>
                                        {if d.active_games.is_empty() {
                                            view! {
                                                <p class="profile-no-games">"No active games."</p>
                                            }.into_any()
                                        } else {
                                            let active_games = d.active_games.clone();
                                            let rows = active_games.into_iter().map(|row| {
                                                let href = format!("/games/{}", row.game_id);
                                                let updated = row.updated_at.date().to_string();
                                                let opponents = opponents_view(row.opponents.clone());
                                                view! {
                                                    <tr class:my-turn=row.is_turn>
                                                        <td><A href=href>{row.game_type_name.clone()}</A></td>
                                                        <td>{opponents}</td>
                                                        <td>{updated}</td>
                                                    </tr>
                                                }
                                            }).collect_view();
                                            view! {
                                                <div class="table-scroll">
                                                    <table>
                                                        <thead>
                                                            <tr>
                                                                <th>"Game"</th>
                                                                <th>"Opponents"</th>
                                                                <th>"Updated"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody>{rows}</tbody>
                                                    </table>
                                                </div>
                                            }.into_any()
                                        }}
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

#[component]
pub fn PlayerGameTypePage() -> impl IntoView {
    use crate::components::layout::MainLayout;

    let params = leptos_router::hooks::use_params_map();
    let query = leptos_router::hooks::use_query_map();
    let data_res = Resource::new_blocking(
        move || {
            (
                params.get().get("name").unwrap_or_default(),
                params.get().get("game_type").unwrap_or_default(),
                query.get().get("bots").as_deref() == Some("1"),
            )
        },
        |(name, game_type, include_single_human)| async move {
            crate::stats::get_player_game_type_stats(name, game_type, include_single_human).await
        },
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
                            <div class="profile content-page">
                                <h1>"Not found"</h1>
                                <p>"No such player or game type."</p>
                            </div>
                        }.into_any(),
                        Ok(Some(d)) => {
                            let name_style = d
                                .user
                                .pref_color
                                .as_ref()
                                .map(|c| format!("color:var(--mk-{})", c.to_lowercase()));
                            let win_rate = if d.stats.games == 0 {
                                "-".to_string()
                            } else {
                                format!("{:.1}%", d.stats.win_percent)
                            };
                            let rating = d
                                .stats
                                .rating
                                .map(|r| r.to_string())
                                .unwrap_or_else(|| "-".to_string());
                            let peak_rating = d
                                .stats
                                .peak_rating
                                .map(|r| r.to_string())
                                .unwrap_or_else(|| "-".to_string());

                            let back_href = {
                                let mut h = format!("/players/{}", encode_path_segment(&d.user.name));
                                if query.get().get("bots").as_deref() == Some("1") {
                                    h.push_str("?bots=1");
                                }
                                h
                            };

                            let histograms = placing_histograms(&d.finished_games);

                            view! {
                                <div class="profile content-page">
                                    <p class="profile-back-link">
                                        <A href=back_href>{format!("< back to {}", d.user.name)}</A>
                                    </p>
                                    <header class="profile-header">
                                        <h1>
                                            <span style=name_style>{d.user.name.clone()}</span>
                                            " - " {d.game_type_name.clone()}
                                        </h1>
                                        <p>
                                            "Rating: " {rating} " (peak " {peak_rating} ") - "
                                            "Games: " {d.stats.games} " - Wins: " {d.stats.wins}
                                            " - Win rate: " {win_rate}
                                        </p>
                                    </header>
                                    <div class="profile-bots-toggle">
                                        {
                                            let toggle_name = d.user.name.clone();
                                            let toggle_game_type = d.game_type_name.clone();
                                            move || {
                                                if query.get().get("bots").as_deref() == Some("1") {
                                                    let href = format!(
                                                        "/players/{}/{}",
                                                        encode_path_segment(&toggle_name),
                                                        encode_path_segment(&toggle_game_type),
                                                    );
                                                    view! {
                                                        <A href=href>
                                                            "Showing bot-only games - exclude them"
                                                        </A>
                                                    }.into_any()
                                                } else {
                                                    let href = format!(
                                                        "/players/{}/{}?bots=1",
                                                        encode_path_segment(&toggle_name),
                                                        encode_path_segment(&toggle_game_type),
                                                    );
                                                    view! {
                                                        <A href=href>"Include bot-only games"</A>
                                                    }.into_any()
                                                }
                                            }
                                        }
                                    </div>
                                    <section class="gt-rating-chart">
                                        <h2>"Rating over time"</h2>
                                        {if d.rating_series.is_empty() {
                                            view! {
                                                <p class="profile-no-games">"No rated games."</p>
                                            }.into_any()
                                        } else {
                                            view! { <RatingChart points=d.rating_series.clone()/> }.into_any()
                                        }}
                                    </section>
                                    <section class="gt-histograms">
                                        <h2>"Placing distribution"</h2>
                                        {if histograms.is_empty() {
                                            view! {
                                                <p class="profile-no-games">"No finished games yet."</p>
                                            }.into_any()
                                        } else {
                                            let items = histograms.into_iter().map(|h| {
                                                let win_pct = format!(
                                                    "{:.1}%",
                                                    h.wins as f64 / h.games as f64 * 100.0,
                                                );
                                                view! {
                                                    <div class="gt-histogram">
                                                        <h3>{h.label.clone()}</h3>
                                                        <Histogram buckets=h.buckets/>
                                                        <p>"Win % " {win_pct}</p>
                                                    </div>
                                                }
                                            }).collect_view();
                                            view! { <div class="gt-histogram-list">{items}</div> }.into_any()
                                        }}
                                    </section>
                                    <section class="gt-finished-games">
                                        <h2>"Finished games"</h2>
                                        {if d.finished_games.is_empty() {
                                            view! {
                                                <p class="profile-no-games">"No finished games yet."</p>
                                            }.into_any()
                                        } else {
                                            let rows = d.finished_games.clone().into_iter().map(|row| {
                                                let href = format!("/games/{}", row.game_id);
                                                let finished = row
                                                    .finished_at
                                                    .map(|t| t.date().to_string())
                                                    .unwrap_or_else(|| "-".to_string());
                                                let placing = format_placing(row.place, row.player_count);
                                                let rating = match row.rating_change {
                                                    None => view! { "-" }.into_any(),
                                                    Some(n) if n > 0 => view! {
                                                        <span class="rating-change-up">{format!("+{n}")}</span>
                                                    }.into_any(),
                                                    Some(n) if n < 0 => view! {
                                                        <span class="rating-change-down">{n.to_string()}</span>
                                                    }.into_any(),
                                                    Some(_) => view! {
                                                        <span class="rating-change-none">"0"</span>
                                                    }.into_any(),
                                                };
                                                let opponents = opponents_view(row.opponents);
                                                view! {
                                                    <tr>
                                                        <td><A href=href>{finished}</A></td>
                                                        <td>{placing}</td>
                                                        <td>{rating}</td>
                                                        <td>{opponents}</td>
                                                    </tr>
                                                }
                                            }).collect_view();
                                            view! {
                                                <div class="table-scroll">
                                                    <table>
                                                        <thead>
                                                            <tr>
                                                                <th>"Finished"</th>
                                                                <th>"Placing"</th>
                                                                <th>"Rating"</th>
                                                                <th>"Opponents"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody>{rows}</tbody>
                                                    </table>
                                                </div>
                                            }.into_any()
                                        }}
                                    </section>
                                    <section class="gt-head-to-head">
                                        <h2>"Head-to-head"</h2>
                                        {if d.head_to_head.is_empty() {
                                            view! {
                                                <p class="profile-no-games">"No head-to-head data yet."</p>
                                            }.into_any()
                                        } else {
                                            let rows = d.head_to_head.clone().into_iter().map(|h| {
                                                let href = format!(
                                                    "/players/{}",
                                                    encode_path_segment(&h.name),
                                                );
                                                view! {
                                                    <tr>
                                                        <td><A href=href>{h.name.clone()}</A></td>
                                                        <td>{h.games}</td>
                                                        <td>{h.wins}</td>
                                                        <td>{h.losses}</td>
                                                        <td>{h.ties}</td>
                                                    </tr>
                                                }
                                            }).collect_view();
                                            view! {
                                                <div class="table-scroll">
                                                    <table>
                                                        <thead>
                                                            <tr>
                                                                <th>"Opponent"</th>
                                                                <th>"Games"</th>
                                                                <th>"Wins"</th>
                                                                <th>"Losses"</th>
                                                                <th>"Ties"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody>{rows}</tbody>
                                                    </table>
                                                </div>
                                            }.into_any()
                                        }}
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn fixture(rating_change: Option<i32>) -> FormResult {
        FormResult {
            game_id: Uuid::new_v4(),
            finished_at: None,
            place: Some(1),
            player_count: 2,
            rating_change,
        }
    }

    fn finished_game(place: Option<i32>, player_count: i64) -> FinishedGameRow {
        FinishedGameRow {
            game_id: Uuid::new_v4(),
            game_type_name: "Camel Up".to_string(),
            finished_at: None,
            place,
            player_count,
            rating_change: None,
            opponents: Vec::new(),
        }
    }

    #[test]
    fn encode_path_segment_passthrough() {
        assert_eq!(encode_path_segment("alice_Bob-1"), "alice_Bob-1");
    }

    #[test]
    fn encode_path_segment_space() {
        assert_eq!(encode_path_segment("Camel Up"), "Camel%20Up");
    }

    #[test]
    fn encode_path_segment_non_ascii() {
        assert_eq!(encode_path_segment("é"), "%C3%A9");
    }

    #[test]
    fn encode_path_segment_slash() {
        assert_eq!(encode_path_segment("a/b"), "a%2Fb");
    }

    #[test]
    fn rating_trend_none_current_is_empty() {
        let results = vec![fixture(Some(16)), fixture(Some(-8))];
        assert_eq!(rating_trend(None, &results), Vec::<f64>::new());
    }

    #[test]
    fn rating_trend_fewer_than_two_changes_is_empty() {
        let results = vec![fixture(Some(16)), fixture(None)];
        assert_eq!(rating_trend(Some(1228), &results), Vec::<f64>::new());
    }

    #[test]
    fn format_placing_none_is_dash() {
        assert_eq!(format_placing(None, 4), "-");
    }

    #[test]
    fn format_placing_ordinals() {
        assert_eq!(format_placing(Some(1), 4), "1st of 4");
        assert_eq!(format_placing(Some(2), 4), "2nd of 4");
        assert_eq!(format_placing(Some(3), 4), "3rd of 4");
        assert_eq!(format_placing(Some(4), 4), "4th of 4");
        assert_eq!(format_placing(Some(11), 20), "11th of 20");
        assert_eq!(format_placing(Some(12), 20), "12th of 20");
        assert_eq!(format_placing(Some(13), 20), "13th of 20");
        assert_eq!(format_placing(Some(21), 21), "21st of 21");
    }

    #[test]
    fn rating_trend_reconstructs_series() {
        let results = vec![
            fixture(Some(16)),
            fixture(None),
            fixture(Some(-8)),
            fixture(Some(20)),
        ];
        assert_eq!(
            rating_trend(Some(1228), &results),
            vec![1216.0, 1208.0, 1228.0]
        );
    }

    #[test]
    fn placing_histograms_empty_input_is_empty() {
        assert_eq!(placing_histograms(&[]), Vec::new());
    }

    #[test]
    fn placing_histograms_two_player_only() {
        let games = vec![
            finished_game(Some(1), 2),
            finished_game(Some(1), 2),
            finished_game(Some(2), 2),
        ];
        let hists = placing_histograms(&games);
        assert_eq!(hists.len(), 1);
        let h = &hists[0];
        assert_eq!(h.label, "2 players");
        assert_eq!(h.games, 3);
        assert_eq!(h.wins, 2);
        assert_eq!(
            h.buckets,
            vec![
                HistogramBucket {
                    label: "1st".to_string(),
                    count: 2
                },
                HistogramBucket {
                    label: "2nd".to_string(),
                    count: 1
                },
            ]
        );
    }

    #[test]
    fn placing_histograms_mixed_player_counts_and_grouping() {
        let games = vec![
            finished_game(Some(1), 2),
            finished_game(Some(2), 3),
            finished_game(Some(1), 3),
            finished_game(Some(3), 3),
            finished_game(Some(4), 5),
            finished_game(Some(1), 4),
        ];
        let hists = placing_histograms(&games);
        let labels: Vec<&str> = hists.iter().map(|h| h.label.as_str()).collect();
        assert_eq!(labels, vec!["2 players", "3 players", "4+ players"]);

        let two = &hists[0];
        assert_eq!(two.games, 1);
        assert_eq!(two.wins, 1);

        let three = &hists[1];
        assert_eq!(three.games, 3);
        assert_eq!(three.wins, 1);
        assert_eq!(
            three.buckets,
            vec![
                HistogramBucket {
                    label: "1st".to_string(),
                    count: 1
                },
                HistogramBucket {
                    label: "2nd".to_string(),
                    count: 1
                },
                HistogramBucket {
                    label: "3rd".to_string(),
                    count: 1
                },
            ]
        );

        let four_plus = &hists[2];
        assert_eq!(four_plus.games, 2);
        assert_eq!(four_plus.wins, 1);
        assert_eq!(
            four_plus.buckets,
            vec![
                HistogramBucket {
                    label: "1st".to_string(),
                    count: 1
                },
                HistogramBucket {
                    label: "2nd".to_string(),
                    count: 0
                },
                HistogramBucket {
                    label: "3rd".to_string(),
                    count: 0
                },
                HistogramBucket {
                    label: "4th".to_string(),
                    count: 1
                },
            ]
        );
    }

    #[test]
    fn placing_histograms_none_place_excluded() {
        let games = vec![finished_game(None, 2), finished_game(None, 2)];
        assert_eq!(placing_histograms(&games), Vec::new());
    }

    #[test]
    fn placing_histograms_none_place_rows_not_counted_alongside_placed() {
        let games = vec![finished_game(Some(1), 2), finished_game(None, 2)];
        let hists = placing_histograms(&games);
        assert_eq!(hists.len(), 1);
        assert_eq!(hists[0].games, 1);
        assert_eq!(hists[0].wins, 1);
    }
}
