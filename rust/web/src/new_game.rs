use leptos::prelude::*;
use leptos_router::{
    NavigateOptions,
    components::A,
    hooks::{use_navigate, use_params_map, use_query_map},
};
use uuid::Uuid;

use crate::components::{OpponentSlot, OpponentSlotEditor};
use crate::game::server_fns::{
    BotSlot, GameTypeInfo, PrefillSlot, RestartOutcome, get_available_bots, get_restart_prefill,
    restart_game_with_roster,
};
use crate::players::encode_path_segment;

/// Formats supported player counts, honoring non-contiguous sets:
/// [2,3,4] -> "2-4 players", [2] -> "2 players", [2,4,6] -> "2, 4, 6 players".
fn player_range(counts: &[i32]) -> String {
    match counts {
        [] => String::new(),
        [n] => format!("{n} players"),
        _ => {
            let contiguous = counts.windows(2).all(|w| w[1] == w[0] + 1);
            if contiguous {
                format!("{}-{} players", counts[0], counts[counts.len() - 1])
            } else {
                let list = counts
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{list} players")
            }
        }
    }
}

fn weight_text(weight: f32) -> String {
    format!("Weight {weight:.1} / 5")
}

fn prefill_to_slots(opponents: &[PrefillSlot]) -> Vec<OpponentSlot> {
    opponents
        .iter()
        .map(|s| match (s.user_id, s.bot_name.as_deref()) {
            (Some(uid), _) => OpponentSlot::Player {
                query: String::new(),
                selected: Some((uid, s.name.clone())),
            },
            (None, Some(bot_name)) => OpponentSlot::Bot {
                name: s.name.clone(),
                bot_name: bot_name.to_string(),
            },
            (None, None) => OpponentSlot::default(),
        })
        .collect()
}

/// Client-side filter + sort over the already-fetched list. `sort_key` is
/// one of "alpha" (default), "weight-asc", "weight-desc"; weight ties break
/// alphabetically.
fn filter_and_sort(
    types: &[GameTypeInfo],
    count_filter: Option<i32>,
    text: &str,
    sort_key: &str,
) -> Vec<GameTypeInfo> {
    let text = text.trim().to_lowercase();
    let mut list: Vec<GameTypeInfo> = types
        .iter()
        .filter(|gt| count_filter.is_none_or(|c| gt.player_counts.contains(&c)))
        .filter(|gt| text.is_empty() || gt.name.to_lowercase().contains(&text))
        .cloned()
        .collect();
    match sort_key {
        "weight-asc" => list.sort_by(|a, b| {
            a.weight
                .partial_cmp(&b.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        }),
        "weight-desc" => list.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        }),
        _ => list.sort_by(|a, b| a.name.cmp(&b.name)),
    }
    list
}

#[component]
pub fn NewGameTypePage() -> impl IntoView {
    use crate::components::layout::MainLayout;
    use crate::game::server_fns::get_available_game_types;

    let game_types = LocalResource::new(get_available_game_types);

    view! {
        <MainLayout>
            <div class="new-game content-page">
                <h1>"New Game"</h1>
                {move || match game_types.get() {
                    None => view! { <p>"Loading..."</p> }.into_any(),
                    Some(Err(e)) => view! { <p class="error">{crate::error::user_facing_server_error(&e)}</p> }.into_any(),
                    Some(Ok(t)) if t.is_empty() => view! { <p>"No games available."</p> }.into_any(),
                    Some(Ok(types)) => view! { <GameTypeGrid types=types/> }.into_any(),
                }}
            </div>
        </MainLayout>
    }
}

#[component]
fn GameTypeGrid(types: Vec<GameTypeInfo>) -> impl IntoView {
    let types = StoredValue::new(types);
    let (filter_players, set_filter_players) = signal(String::new());
    let (filter_text, set_filter_text) = signal(String::new());
    let (sort_key, set_sort_key) = signal("alpha".to_string());

    let visible_types = Memo::new(move |_| {
        types.with_value(|t| {
            filter_and_sort(
                t,
                filter_players.get().trim().parse::<i32>().ok(),
                &filter_text.get(),
                &sort_key.get(),
            )
        })
    });

    view! {
        <div class="new-game-browser">
            <div class="new-game-filters">
                <input
                    type="number"
                    min="1"
                    class="new-game-filter-players"
                    placeholder="Players"
                    aria-label="Filter by player count"
                    prop:value=filter_players
                    on:input=move |ev| set_filter_players.set(event_target_value(&ev))
                />
                <input
                    type="search"
                    class="new-game-filter-search"
                    placeholder="Search games"
                    aria-label="Search games by name"
                    prop:value=filter_text
                    on:input=move |ev| set_filter_text.set(event_target_value(&ev))
                />
                <select
                    aria-label="Sort games"
                    on:change=move |ev| set_sort_key.set(event_target_value(&ev))
                >
                    <option value="alpha">"Alphabetical"</option>
                    <option value="weight-asc">"Weight (low to high)"</option>
                    <option value="weight-desc">"Weight (high to low)"</option>
                </select>
            </div>
            <div class="game-card-grid">
                <For
                    each=move || visible_types.get()
                    key=|gt| gt.id
                    children=move |gt: GameTypeInfo| {
                        let href = format!("/games/new/{}", encode_path_segment(&gt.name));
                        let name = gt.name.clone();
                        let meta = format!(
                            "{} | {}",
                            player_range(&gt.player_counts),
                            weight_text(gt.weight)
                        );
                        let blurb = gt.blurb.clone();
                        view! {
                            <A href=href attr:class="game-card">
                                <span class="game-card-name">{name}</span>
                                <span class="game-card-meta">{meta}</span>
                                {(!blurb.is_empty())
                                    .then(|| view! { <span class="game-card-blurb">{blurb.clone()}</span> })}
                            </A>
                        }
                    }
                />
            </div>
        </div>
    }
}

#[component]
pub fn NewGameSetupPage() -> impl IntoView {
    use crate::components::layout::MainLayout;
    use crate::game::server_fns::get_available_game_types;

    let params = use_params_map();
    let restart_game_id: Option<Uuid> = use_query_map()
        .get()
        .get("restart")
        .and_then(|s| s.parse::<Uuid>().ok());
    let game_types = LocalResource::new(get_available_game_types);

    view! {
        <MainLayout>
            <div class="new-game content-page">
                <p class="new-game-back">
                    <A href="/games/new">"Back to games"</A>
                </p>
                {move || {
                    let wanted = params.get().get("type").unwrap_or_default();
                    match game_types.get() {
                        None => view! { <p>"Loading..."</p> }.into_any(),
                        Some(Err(e)) => view! { <p class="error">{crate::error::user_facing_server_error(&e)}</p> }.into_any(),
                        Some(Ok(types)) => match types
                            .iter()
                            .find(|gt| gt.name.eq_ignore_ascii_case(&wanted))
                        {
                            None => view! {
                                <h1>"Game not found"</h1>
                                <p>"No such game type."</p>
                            }
                                .into_any(),
                            Some(gt) => view! { <GameSetupPanel gt=gt.clone() restart=restart_game_id/> }
                                .into_any(),
                        },
                    }
                }}
            </div>
        </MainLayout>
    }
}

#[component]
fn GameSetupPanel(gt: GameTypeInfo, restart: Option<Uuid>) -> impl IntoView {
    let suggestions = LocalResource::new(crate::friends::get_opponent_suggestions);
    let bot_names = LocalResource::new(get_available_bots);
    let prefill = LocalResource::new(move || async move {
        match restart {
            Some(game_id) => Some(get_restart_prefill(game_id).await),
            None => None,
        }
    });

    let (selected_version_id, set_selected_version_id) = signal(gt.versions.first().map(|v| v.id));
    let (player_count, set_player_count) = signal(gt.player_counts.first().copied().unwrap_or(2));
    let (opponent_slots, set_opponent_slots) = signal(Vec::<OpponentSlot>::new());
    let (form_error, set_form_error) = signal(None::<String>);

    // Users already taken by any slot never appear as chips again.
    let taken = Signal::derive(move || -> Vec<Uuid> {
        opponent_slots
            .get()
            .iter()
            .filter_map(|s| match s {
                OpponentSlot::Player {
                    selected: Some((id, _)),
                    ..
                } => Some(*id),
                _ => None,
            })
            .collect()
    });

    // Opponent slots track player_count - 1. Existing slot state survives
    // count changes where possible (resize, not rebuild).
    Effect::new(move |_| {
        let n = (player_count.get() - 1).max(0) as usize;
        set_opponent_slots.update(|v| v.resize_with(n, OpponentSlot::default));
    });

    Effect::new(move |_| {
        let Some(Some(Ok(pf))) = prefill.get() else {
            return;
        };
        let count = (pf.opponents.len() + 1) as i32;
        let slots = prefill_to_slots(&pf.opponents);
        set_selected_version_id.set(Some(pf.version_id));
        set_opponent_slots.set(slots);
        set_player_count.set(count);
    });

    let create_action = Action::new(
        |(version_id, ids, emails, bots): &(Uuid, Vec<Uuid>, Vec<String>, Vec<BotSlot>)| {
            let version_id = *version_id;
            let ids = ids.clone();
            let emails = emails.clone();
            let bots = bots.clone();
            async move {
                crate::proposals::create_proposal(version_id, Some(ids), Some(emails), Some(bots))
                    .await
            }
        },
    );

    let restart_action = Action::new(
        |(game_id, version_id, ids, emails, bots): &(
            Uuid,
            Uuid,
            Vec<Uuid>,
            Vec<String>,
            Vec<BotSlot>,
        )| {
            let game_id = *game_id;
            let version_id = *version_id;
            let ids = ids.clone();
            let emails = emails.clone();
            let bots = bots.clone();
            async move {
                restart_game_with_roster(game_id, version_id, Some(ids), Some(emails), Some(bots))
                    .await
            }
        },
    );

    let navigate = use_navigate();
    let navigate_create = navigate.clone();
    Effect::new(move |_| {
        if let Some(Ok(outcome)) = create_action.value().get() {
            if let Some(gid) = outcome.game_id {
                navigate_create(&format!("/games/{}", gid), NavigateOptions::default());
            } else if let Some(pid) = outcome.proposal_id {
                navigate_create(&format!("/invites/{}", pid), NavigateOptions::default());
            }
        }
    });

    let navigate_restart = navigate.clone();
    Effect::new(move |_| {
        if let Some(Ok(outcome)) = restart_action.value().get() {
            match outcome {
                RestartOutcome::Created(po) => {
                    if let Some(gid) = po.game_id {
                        navigate_restart(&format!("/games/{gid}"), NavigateOptions::default());
                    } else if let Some(pid) = po.proposal_id {
                        navigate_restart(&format!("/invites/{pid}"), NavigateOptions::default());
                    }
                }
                RestartOutcome::AlreadyRestarted {
                    game_id: Some(g), ..
                } => {
                    navigate_restart(&format!("/games/{g}"), NavigateOptions::default());
                }
                RestartOutcome::AlreadyRestarted {
                    proposal_id: Some(p),
                    ..
                } => {
                    navigate_restart(&format!("/invites/{p}"), NavigateOptions::default());
                }
                RestartOutcome::AlreadyRestarted { .. } => {}
            }
        }
    });

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(version_id) = selected_version_id.get_untracked() else {
            return;
        };
        let mut ids = Vec::new();
        let mut emails = Vec::new();
        let mut bots = Vec::new();
        for slot in opponent_slots.get_untracked() {
            match slot {
                OpponentSlot::Player {
                    selected: Some((id, _)),
                    ..
                } => ids.push(id),
                OpponentSlot::Player { selected: None, .. } => {
                    set_form_error.set(Some(
                        "Choose a player for each Player slot, or switch the slot to Email or Bot"
                            .to_string(),
                    ));
                    return;
                }
                OpponentSlot::Email(email) => emails.push(email),
                OpponentSlot::Bot { name, bot_name } => bots.push(BotSlot { name, bot_name }),
            }
        }
        set_form_error.set(None);
        if let Some(game_id) = restart {
            restart_action.dispatch((game_id, version_id, ids, emails, bots));
        } else {
            create_action.dispatch((version_id, ids, emails, bots));
        }
    };

    let server_error = move || -> Option<ServerFnError> {
        if restart.is_some() {
            restart_action.value().get().and_then(|r| r.err())
        } else {
            create_action.value().get().and_then(|r| r.err())
        }
    };

    let gt = StoredValue::new(gt);

    view! {
        <div class="new-game-panel">
            <h2>
                {gt.with_value(|g| match restart {
                    Some(_) => format!("Restarting {}", g.name),
                    None => g.name.clone(),
                })}
            </h2>
            <p class="game-card-meta">
                {gt.with_value(|g| player_range(&g.player_counts))} " | "
                {gt.with_value(|g| weight_text(g.weight))}
            </p>
            {gt.with_value(|g| {
                (!g.blurb.is_empty())
                    .then(|| view! { <p class="new-game-blurb">{g.blurb.clone()}</p> })
            })}
            {restart.map(|gid| view! {
                <p class="new-game-back">
                    <A href=format!("/games/{gid}")>"Back to finished game"</A>
                </p>
            })}
            <form on:submit=on_submit>
                {gt.with_value(|g| {
                    (restart.is_none() && g.versions.len() > 1).then(|| {
                        let versions = g.versions.clone();
                        view! {
                            <div class="form-field">
                                <label class="form-label">"Version"</label>
                                <div class="form-control">
                                    <select
                                        aria-label="Version"
                                        on:change=move |ev| {
                                            set_selected_version_id
                                                .set(event_target_value(&ev).parse::<Uuid>().ok());
                                        }
                                    >
                                        {versions
                                            .iter()
                                            .map(|v| {
                                                let vid = v.id;
                                                view! {
                                                    <option
                                                        value=vid.to_string()
                                                        selected=move || {
                                                            selected_version_id.get() == Some(vid)
                                                        }
                                                    >
                                                        {v.name.clone()}
                                                    </option>
                                                }
                                            })
                                            .collect_view()}
                                    </select>
                                </div>
                            </div>
                        }
                    })
                })}
                {move || {
                    selected_version_id.get().map(|vid| {
                        view! {
                            <div class="form-field new-game-rules-link">
                                <A href=format!("/rules/{}", vid)>"View rules"</A>
                            </div>
                        }
                    })
                }}
                <div class="form-field">
                    <label class="form-label">"Players"</label>
                    <div
                        class="form-control player-count-radios"
                        role="radiogroup"
                        aria-label="Number of players"
                    >
                        {gt.with_value(|g| g.player_counts.clone())
                            .into_iter()
                            .map(|n| {
                                view! {
                                    <label>
                                        <input
                                            type="radio"
                                            name="player-count"
                                            prop:checked=move || player_count.get() == n
                                            on:change=move |_| set_player_count.set(n)
                                        />
                                        " "
                                        {n}
                                    </label>
                                }
                            })
                            .collect_view()}
                    </div>
                </div>
                {move || {
                    let n = (player_count.get() - 1).max(0) as usize;
                    (0..n)
                        .map(|i| {
                            let get = Signal::derive(move || {
                                opponent_slots
                                    .get()
                                    .get(i)
                                    .cloned()
                                    .unwrap_or_default()
                            });
                            let set = Callback::new(move |s: OpponentSlot| {
                                set_opponent_slots.update(|v| {
                                    if let Some(slot) = v.get_mut(i) {
                                        *slot = s;
                                    }
                                });
                            });
                            view! {
                                <OpponentSlotEditor
                                    label=format!("Opponent {}", i + 1)
                                    radio_group=format!("slot-mode-{i}")
                                    bot_default_name=format!("Bot {}", i + 1)
                                    get=get
                                    set=set
                                    taken=taken
                                    suggestions=suggestions
                                    bot_names=bot_names
                                />
                            }
                        })
                        .collect_view()
                }}
                <div class="form-actions">
                    <input
                        type="submit"
                        value=if restart.is_some() { "Restart game" } else { "Start game" }
                        disabled=move || create_action.pending().get() || restart_action.pending().get()
                    />
                    <Show when=move || create_action.pending().get() || restart_action.pending().get()>
                        <crate::components::Spinner/>
                    </Show>
                </div>
                {move || {
                    form_error
                        .get()
                        .map(|e| view! { <div class="form-error">{e}</div> })
                }}
                <Show when=move || server_error().is_some()>
                    <div class="form-error">
                        {move || {
                            server_error()
                                .map(|e| crate::error::user_facing_server_error(&e))
                                .unwrap_or_default()
                        }}
                    </div>
                </Show>
            </form>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::server_fns::GameTypeInfo;
    use uuid::Uuid;

    fn gt(name: &str, counts: &[i32], weight: f32) -> GameTypeInfo {
        GameTypeInfo {
            id: Uuid::new_v4(),
            name: name.to_string(),
            player_counts: counts.to_vec(),
            weight,
            blurb: String::new(),
            versions: Vec::new(),
        }
    }

    fn names(list: &[GameTypeInfo]) -> Vec<&str> {
        list.iter().map(|g| g.name.as_str()).collect()
    }

    #[test]
    fn prefill_to_slots_maps_humans_and_bots() {
        use crate::game::server_fns::PrefillSlot;
        let uid = Uuid::new_v4();
        let opponents = vec![
            PrefillSlot {
                user_id: Some(uid),
                name: "alice".to_string(),
                bot_name: None,
            },
            PrefillSlot {
                user_id: None,
                name: "Botty".to_string(),
                bot_name: Some("easy".to_string()),
            },
        ];
        let slots = prefill_to_slots(&opponents);
        assert_eq!(slots.len(), 2);
        assert!(matches!(
            &slots[0],
            OpponentSlot::Player { selected: Some((id, name)), .. } if *id == uid && name == "alice"
        ));
        assert!(matches!(
            &slots[1],
            OpponentSlot::Bot { name, bot_name } if name == "Botty" && bot_name == "easy"
        ));
    }

    #[test]
    fn player_range_formats() {
        assert_eq!(player_range(&[2]), "2 players");
        assert_eq!(player_range(&[2, 3, 4]), "2-4 players");
        assert_eq!(player_range(&[2, 4, 6]), "2, 4, 6 players");
        assert_eq!(player_range(&[]), "");
    }

    #[test]
    fn weight_text_formats() {
        assert_eq!(weight_text(2.5), "Weight 2.5 / 5");
        assert_eq!(weight_text(1.0), "Weight 1.0 / 5");
    }

    #[test]
    fn filter_by_player_count() {
        let types = vec![gt("Duel", &[2], 1.0), gt("Party", &[3, 4, 5], 1.0)];
        assert_eq!(
            names(&filter_and_sort(&types, Some(2), "", "alpha")),
            ["Duel"]
        );
        assert_eq!(
            names(&filter_and_sort(&types, Some(4), "", "alpha")),
            ["Party"]
        );
        assert!(filter_and_sort(&types, Some(9), "", "alpha").is_empty());
        // Cleared filter shows all.
        assert_eq!(filter_and_sort(&types, None, "", "alpha").len(), 2);
    }

    #[test]
    fn filter_by_text_is_case_insensitive_substring() {
        let types = vec![gt("Acquire", &[2], 1.0), gt("Lost Cities", &[2], 1.0)];
        assert_eq!(
            names(&filter_and_sort(&types, None, "cIt", "alpha")),
            ["Lost Cities"]
        );
        assert_eq!(filter_and_sort(&types, None, "  ", "alpha").len(), 2);
    }

    #[test]
    fn sort_variants() {
        let types = vec![
            gt("Beta", &[2], 3.0),
            gt("Alpha", &[2], 2.0),
            gt("Gamma", &[2], 2.0),
        ];
        assert_eq!(
            names(&filter_and_sort(&types, None, "", "alpha")),
            ["Alpha", "Beta", "Gamma"]
        );
        assert_eq!(
            names(&filter_and_sort(&types, None, "", "weight-asc")),
            ["Alpha", "Gamma", "Beta"]
        );
        assert_eq!(
            names(&filter_and_sort(&types, None, "", "weight-desc")),
            ["Beta", "Alpha", "Gamma"]
        );
    }
}
