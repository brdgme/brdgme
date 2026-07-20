use gloo_timers::future::TimeoutFuture;
use leptos::html;
use leptos::prelude::*;
use leptos_router::{NavigateOptions, hooks::use_navigate};
use uuid::Uuid;

use crate::friends::{OpponentSuggestion, UserSearchResult};
use crate::game::server_fns::{
    BotSlot, GameTypeInfo, create_new_game, generate_bot_name, get_available_bots,
};

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

/// True below the 60em breakpoint (single-column layout), where selecting a
/// game should scroll the setup panel into view.
fn is_narrow() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(max-width: 60em)").ok().flatten())
        .map(|m| m.matches())
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SlotMode {
    Player,
    Email,
    Bot,
}

/// Per-opponent slot state. Player = a site user, picked via suggestion
/// chip or typeahead; Email = invite by address; Bot = name + bot_name.
#[derive(Debug, Clone)]
enum OpponentSlot {
    Player {
        query: String,
        selected: Option<(Uuid, String)>,
    },
    Email(String),
    Bot {
        name: String,
        bot_name: String,
    },
}

impl OpponentSlot {
    fn mode(&self) -> SlotMode {
        match self {
            OpponentSlot::Player { .. } => SlotMode::Player,
            OpponentSlot::Email(_) => SlotMode::Email,
            OpponentSlot::Bot { .. } => SlotMode::Bot,
        }
    }
}

impl Default for OpponentSlot {
    fn default() -> Self {
        OpponentSlot::Player {
            query: String::new(),
            selected: None,
        }
    }
}

#[component]
pub fn NewGamePage() -> impl IntoView {
    use crate::components::layout::MainLayout;
    use crate::game::server_fns::get_available_game_types;

    let game_types = LocalResource::new(get_available_game_types);

    view! {
        <MainLayout>
            <div class="new-game content-page">
                <h1>"New Game"</h1>
                {move || match game_types.get() {
                    None => view! { <p>"Loading..."</p> }.into_any(),
                    Some(Err(e)) => view! { <p class="error">"Error: " {e.to_string()}</p> }.into_any(),
                    Some(Ok(t)) if t.is_empty() => view! { <p>"No games available."</p> }.into_any(),
                    Some(Ok(types)) => view! { <GameBrowser types=types/> }.into_any(),
                }}
            </div>
        </MainLayout>
    }
}

#[component]
fn GameBrowser(types: Vec<GameTypeInfo>) -> impl IntoView {
    let types = StoredValue::new(types);
    let suggestions = LocalResource::new(crate::friends::get_opponent_suggestions);
    let bot_names = LocalResource::new(get_available_bots);

    let (selected_type_id, set_selected_type_id) = signal(None::<Uuid>);
    let (selected_version_id, set_selected_version_id) = signal(None::<Uuid>);
    let (player_count, set_player_count) = signal(0i32);
    let (opponent_slots, set_opponent_slots) = signal(Vec::<OpponentSlot>::new());

    let (filter_players, set_filter_players) = signal(String::new());
    let (filter_text, set_filter_text) = signal(String::new());
    let (sort_key, set_sort_key) = signal("alpha".to_string());
    let (form_error, set_form_error) = signal(None::<String>);

    let panel_ref = NodeRef::<html::Div>::new();

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

    // Filtering out the selected game deselects it: the panel returns to
    // its empty state.
    Effect::new(move |_| {
        if let Some(id) = selected_type_id.get()
            && !visible_types.get().iter().any(|gt| gt.id == id)
        {
            set_selected_type_id.set(None);
            set_selected_version_id.set(None);
        }
    });

    // Opponent slots track player_count - 1. Existing slot state survives
    // count changes where possible (resize, not rebuild).
    Effect::new(move |_| {
        let n = (player_count.get() - 1).max(0) as usize;
        set_opponent_slots.update(|v| v.resize_with(n, OpponentSlot::default));
    });

    let select_game = move |gt: &GameTypeInfo| {
        set_selected_type_id.set(Some(gt.id));
        set_selected_version_id.set(gt.versions.first().map(|v| v.id));
        set_player_count.set(gt.player_counts.first().copied().unwrap_or(2));
        set_form_error.set(None);
        // Single-column layout: bring the setup panel into view.
        if is_narrow()
            && let Some(el) = panel_ref.get_untracked()
        {
            el.scroll_into_view();
        }
    };

    let create_action = Action::new(
        |(version_id, ids, emails, bots): &(Uuid, Vec<Uuid>, Vec<String>, Vec<BotSlot>)| {
            let version_id = *version_id;
            let ids = if ids.is_empty() {
                None
            } else {
                Some(ids.clone())
            };
            let emails = if emails.is_empty() {
                None
            } else {
                Some(emails.clone())
            };
            let bots = if bots.is_empty() {
                None
            } else {
                Some(bots.clone())
            };
            async move { create_new_game(version_id, ids, emails, bots).await }
        },
    );

    let navigate = use_navigate();
    Effect::new(move |_| {
        if let Some(Ok(id)) = create_action.value().get() {
            navigate(&format!("/games/{}", id), NavigateOptions::default());
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
        create_action.dispatch((version_id, ids, emails, bots));
    };

    view! {
        <div class="new-game-layout">
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
                            let id = gt.id;
                            let name = gt.name.clone();
                            let meta = format!(
                                "{} | {}",
                                player_range(&gt.player_counts),
                                weight_text(gt.weight)
                            );
                            let blurb = gt.blurb.clone();
                            view! {
                                <label class="game-card">
                                    <input
                                        type="radio"
                                        name="game-type"
                                        class="sr-only"
                                        prop:checked=move || selected_type_id.get() == Some(id)
                                        on:change=move |_| select_game(&gt)
                                    />
                                    <span class="game-card-name">{name}</span>
                                    <span class="game-card-meta">{meta}</span>
                                    {(!blurb.is_empty())
                                        .then(|| view! { <span class="game-card-blurb">{blurb.clone()}</span> })}
                                </label>
                            }
                        }
                    />
                </div>
            </div>
            <div class="new-game-panel" node_ref=panel_ref>
                {move || {
                    let Some(gt) = selected_type_id
                        .get()
                        .and_then(|id| {
                            types.with_value(|t| t.iter().find(|g| g.id == id).cloned())
                        })
                    else {
                        return view! {
                            <p class="new-game-panel-empty">
                                "Select a game on the left to set up a match."
                            </p>
                        }
                        .into_any();
                    };
                    let version_select = (gt.versions.len() > 1).then(|| {
                        let versions = gt.versions.clone();
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
                    });
                    let counts = gt.player_counts.clone();
                    view! {
                        <h2>{gt.name.clone()}</h2>
                        <p class="game-card-meta">
                            {player_range(&gt.player_counts)} " | " {weight_text(gt.weight)}
                        </p>
                        {(!gt.blurb.is_empty())
                            .then(|| view! { <p class="new-game-blurb">{gt.blurb.clone()}</p> })}
                        <form on:submit=on_submit>
                            {version_select}
                            <div class="form-field">
                                <label class="form-label">"Players"</label>
                                <div
                                    class="form-control player-count-radios"
                                    role="radiogroup"
                                    aria-label="Number of players"
                                >
                                    {counts
                                        .iter()
                                        .map(|&n| {
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
                                        view! {
                                            <OpponentSlotEditor
                                                i=i
                                                slots=opponent_slots
                                                set_slots=set_opponent_slots
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
                                    value="Start game"
                                    disabled=move || create_action.pending().get()
                                />
                            </div>
                            {move || {
                                form_error
                                    .get()
                                    .map(|e| view! { <div class="form-error">{e}</div> })
                            }}
                            <Show when=move || {
                                create_action.value().get().is_some_and(|r| r.is_err())
                            }>
                                <div class="form-error">
                                    {move || {
                                        create_action
                                            .value()
                                            .get()
                                            .and_then(|r| r.err())
                                            .map(|e| e.to_string())
                                            .unwrap_or_default()
                                    }}
                                </div>
                            </Show>
                        </form>
                    }
                    .into_any()
                }}
            </div>
        </div>
    }
}

#[component]
fn OpponentSlotEditor(
    i: usize,
    slots: ReadSignal<Vec<OpponentSlot>>,
    set_slots: WriteSignal<Vec<OpponentSlot>>,
    suggestions: LocalResource<Result<Vec<OpponentSuggestion>, ServerFnError>>,
    bot_names: LocalResource<Result<Vec<String>, ServerFnError>>,
) -> impl IntoView {
    let slot = move || slots.get().get(i).cloned().unwrap_or_default();
    let mode = move || slot().mode();

    let set_mode = move |m: SlotMode| {
        set_slots.update(|v| {
            if let Some(s) = v.get_mut(i) {
                *s = match m {
                    SlotMode::Player => OpponentSlot::default(),
                    SlotMode::Email => OpponentSlot::Email(String::new()),
                    SlotMode::Bot => OpponentSlot::Bot {
                        name: format!("Bot {}", i + 1),
                        bot_name: "medium".to_string(),
                    },
                };
            }
        });
        if m == SlotMode::Bot {
            leptos::task::spawn_local(async move {
                if let Ok(name) = generate_bot_name().await {
                    set_slots.update(|v| {
                        if let Some(OpponentSlot::Bot { name: bot_name, .. }) = v.get_mut(i) {
                            *bot_name = name;
                        }
                    });
                }
            });
        }
    };

    let pick_user = move |id: Uuid, name: String| {
        set_slots.update(|v| {
            if let Some(s) = v.get_mut(i) {
                *s = OpponentSlot::Player {
                    query: String::new(),
                    selected: Some((id, name.clone())),
                };
            }
        });
    };

    // Users already taken by other slots never appear as chips again.
    let taken = move || -> Vec<Uuid> {
        slots
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
    };

    let slot_query = move || match slot() {
        OpponentSlot::Player { query, .. } => query,
        _ => String::new(),
    };

    // One search action per slot. Each response is tagged with the query that
    // produced it; results and errors are only used when the tag matches the
    // current query, so a slow stale response can never show wrong results.
    let search_action = Action::new(|q: &String| {
        let q = q.clone();
        let tag = q.clone();
        async move { (tag, crate::friends::search_users(q).await) }
    });

    let search_results = move || -> Vec<UserSearchResult> {
        let current = slot_query();
        if current.trim().chars().count() < 2 {
            return Vec::new();
        }
        match search_action.value().get() {
            Some((tag, Ok(results))) if *tag == current => results,
            _ => Vec::new(),
        }
    };

    // Spec, error handling: search failure is inline under the slot; the
    // slot stays usable via Email mode.
    let search_error = move || -> Option<String> {
        let current = slot_query();
        if current.trim().chars().count() < 2 {
            return None;
        }
        match search_action.value().get() {
            Some((tag, Err(e))) if *tag == current => Some(format!("Search failed: {e}")),
            _ => None,
        }
    };

    let (search_seq, set_search_seq) = signal(0u32);

    view! {
        <div class="form-field opponent-slot">
            <label class="form-label">"Opponent " {i + 1}</label>
            <div
                class="form-control slot-modes"
                role="radiogroup"
                aria-label=format!("Opponent {} type", i + 1)
            >
                <label>
                    <input
                        type="radio"
                        name=format!("slot-mode-{i}")
                        prop:checked=move || mode() == SlotMode::Player
                        on:change=move |_| set_mode(SlotMode::Player)
                    />
                    " Player"
                </label>
                <label>
                    <input
                        type="radio"
                        name=format!("slot-mode-{i}")
                        prop:checked=move || mode() == SlotMode::Email
                        on:change=move |_| set_mode(SlotMode::Email)
                    />
                    " Email"
                </label>
                <label>
                    <input
                        type="radio"
                        name=format!("slot-mode-{i}")
                        prop:checked=move || mode() == SlotMode::Bot
                        on:change=move |_| set_mode(SlotMode::Bot)
                    />
                    " Bot"
                </label>
            </div>
            <Show when=move || {
                matches!(slot(), OpponentSlot::Player { selected: Some(_), .. })
            }>
                <div class="form-control">
                    <span class="chip chip-selected">
                        {move || match slot() {
                            OpponentSlot::Player {
                                selected: Some((_, name)),
                                ..
                            } => name,
                            _ => String::new(),
                        }}
                        " "
                        <button
                            type="button"
                            aria-label="Remove player"
                            on:click=move |_| set_mode(SlotMode::Player)
                        >
                            "x"
                        </button>
                    </span>
                </div>
            </Show>
            <Show when=move || {
                matches!(slot(), OpponentSlot::Player { selected: None, .. })
            }>
                <div class="form-control">
                    <input
                        type="text"
                        placeholder="Search players"
                        aria-label=format!("Search players for opponent {}", i + 1)
                        prop:value=slot_query
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            set_slots.update(|v| {
                                if let Some(s) = v.get_mut(i) {
                                    *s = OpponentSlot::Player {
                                        query: val.clone(),
                                        selected: None,
                                    };
                                }
                            });
                            let seq = set_search_seq
                                .try_update(|s| {
                                    *s += 1;
                                    *s
                                })
                                .unwrap_or(0);
                            let q = val.clone();
                            leptos::task::spawn_local(async move {
                                TimeoutFuture::new(300).await;
                                if search_seq.get_untracked() == seq
                                    && q.trim().chars().count() >= 2
                                {
                                    search_action.dispatch(q);
                                }
                            });
                        }
                    />
                </div>
                {move || {
                    search_error()
                        .map(|e| view! { <div class="form-error">{e}</div> })
                }}
                <ul class="typeahead-results">
                    {move || {
                        let tk = taken();
                        search_results()
                            .into_iter()
                            .filter(|r| !tk.contains(&r.user_id))
                            .map(|r| {
                                let id = r.user_id;
                                let name = r.name.clone();
                                let label = r.name.clone();
                                view! {
                                    <li>
                                        <a
                                            href="#"
                                            class="chip"
                                            on:click=move |ev| {
                                                ev.prevent_default();
                                                pick_user(id, name.clone());
                                            }
                                        >
                                            {label}
                                        </a>
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
                <Show when=move || slot_query().is_empty()>
                    <div class="form-control chip-row">
                        {move || {
                            match suggestions.get() {
                                Some(Ok(sugs)) if !sugs.is_empty() => {
                                    let tk = taken();
                                    sugs.iter()
                                        .filter(|s| !tk.contains(&s.user_id))
                                        .map(|s| {
                                            let id = s.user_id;
                                            let name = s.name.clone();
                                            let label = s.name.clone();
                                            view! {
                                                <a
                                                    href="#"
                                                    class="chip"
                                                    class:chip-friend=s.is_friend
                                                    on:click=move |ev| {
                                                        ev.prevent_default();
                                                        pick_user(id, name.clone());
                                                    }
                                                >
                                                    {label}
                                                </a>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                }
                                _ => ().into_any(),
                            }
                        }}
                    </div>
                </Show>
            </Show>
            <Show when=move || mode() == SlotMode::Email>
                <div class="form-control">
                    <input
                        type="email"
                        placeholder="Email address"
                        aria-label="Email address"
                        required
                        prop:value=move || match slot() {
                            OpponentSlot::Email(e) => e,
                            _ => String::new(),
                        }
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            set_slots.update(|v| {
                                if let Some(s) = v.get_mut(i) {
                                    *s = OpponentSlot::Email(val);
                                }
                            });
                        }
                    />
                </div>
            </Show>
            <Show when=move || mode() == SlotMode::Bot>
                <div class="form-control">
                    <select
                        aria-label="Bot difficulty"
                        prop:value=move || match slot() {
                            OpponentSlot::Bot { bot_name, .. } => bot_name,
                            _ => "medium".to_string(),
                        }
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            set_slots.update(|v| {
                                if let Some(OpponentSlot::Bot { bot_name, .. }) = v.get_mut(i) {
                                    *bot_name = val;
                                }
                            });
                        }
                    >
                        {move || {
                            let names = match bot_names.get() {
                                Some(Ok(b)) if !b.is_empty() => b,
                                _ => vec![
                                    "easy".to_string(),
                                    "medium".to_string(),
                                    "hard".to_string(),
                                ],
                            };
                            names
                                .into_iter()
                                .map(|n| {
                                    let label = n.clone();
                                    view! {
                                        <option value=n>{label}</option>
                                    }
                                })
                                .collect_view()
                        }}
                    </select>
                </div>
            </Show>
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
