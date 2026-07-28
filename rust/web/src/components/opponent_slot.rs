use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use uuid::Uuid;

use crate::friends::{OpponentSuggestion, UserSearchResult};
use crate::game::server_fns::generate_bot_name;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlotMode {
    Player,
    Email,
    Bot,
}

/// Per-opponent slot state. Player = a site user, picked via suggestion
/// chip or typeahead; Email = invite by address; Bot = name + bot_name.
#[derive(Debug, Clone)]
pub enum OpponentSlot {
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
    pub fn mode(&self) -> SlotMode {
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

/// A single player-input slot. Operates on one [`OpponentSlot`] via `get`/`set`
/// so it can back a multi-slot list (new game, restart) or a lone "add player"
/// input (pending page). `taken` is the set of user ids the parent has already
/// claimed, used to dedupe suggestions and search results.
#[component]
pub fn OpponentSlotEditor(
    label: String,
    radio_group: String,
    bot_default_name: String,
    get: Signal<OpponentSlot>,
    set: Callback<OpponentSlot>,
    taken: Signal<Vec<Uuid>>,
    suggestions: LocalResource<Result<Vec<OpponentSuggestion>, ServerFnError>>,
    bot_names: LocalResource<Result<Vec<String>, ServerFnError>>,
) -> impl IntoView {
    let bot_default_name = StoredValue::new(bot_default_name);

    let slot = move || get.get();
    let mode = move || slot().mode();

    // wfe F58, half 1: the <select> below must be built exactly once, with an
    // option list that never changes afterwards. When `bot_names` resolves,
    // `collect_view()`'s Vec rebuild rewrites the existing <option> elements'
    // `value` attributes in place while the DOM's selectedness stays on the
    // same element - so the visible choice silently diverges from state. The
    // `prop:value` RenderEffect does not re-run (it tracks `slot()`, not
    // `bot_names`), and making it track `bot_names` would not help: attributes
    // are built before children (tachys html/element/mod.rs:352 vs :357) and a
    // reactive prop's RenderEffect runs immediately on creation
    // (reactive_graph effect/render_effect.rs:61-62). docs/CODING.md also
    // forbids per-<option> `selected=`. So: gate the element on the *settled*
    // resource, and its option list is fixed for its whole lifetime.
    //
    // `None` while the resource is still loading; after that the list is
    // fixed. `bot_names` is created once per page (new_game.rs:235) and never
    // refetched, so this transitions exactly once.
    let bot_name_options = move || -> Option<Vec<String>> {
        let settled = bot_names.get()?;
        Some(match settled {
            Ok(b) if !b.is_empty() => b,
            _ => vec!["easy".to_string(), "medium".to_string(), "hard".to_string()],
        })
    };

    let default_bot_name = move || -> String {
        bot_name_options()
            .and_then(|names| names.into_iter().next())
            .unwrap_or_else(|| "medium".to_string())
    };

    let set_mode = move |m: SlotMode| {
        set.run(match m {
            SlotMode::Player => OpponentSlot::default(),
            SlotMode::Email => OpponentSlot::Email(String::new()),
            SlotMode::Bot => OpponentSlot::Bot {
                name: bot_default_name.with_value(|n| n.clone()),
                bot_name: default_bot_name(),
            },
        });
        if m == SlotMode::Bot {
            leptos::task::spawn_local(async move {
                if let Ok(name) = generate_bot_name().await
                    && let OpponentSlot::Bot { bot_name, .. } = get.get_untracked()
                {
                    set.run(OpponentSlot::Bot { name, bot_name });
                }
            });
        }
    };

    let pick_user = move |id: Uuid, name: String| {
        set.run(OpponentSlot::Player {
            query: String::new(),
            selected: Some((id, name)),
        });
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
            Some((tag, Err(e))) if *tag == current => Some(format!(
                "Search failed: {}",
                crate::error::action_error_message(&e)
            )),
            _ => None,
        }
    };

    let (search_seq, set_search_seq) = signal(0u32);

    // The single source of truth for what the difficulty select should show.
    // Used by both `prop:value` and the post-mount effect below, so the two
    // cannot drift.
    let bot_name_value = move || match slot() {
        OpponentSlot::Bot { bot_name, .. } => bot_name,
        _ => default_bot_name(),
    };

    // wfe F58, half 2: `prop:value` alone can never select anything on first
    // build. The property is written during `attributes.build`
    // (tachys html/element/mod.rs:352) BEFORE `children.build` (:357), and the
    // reactive prop's RenderEffect runs immediately, so it targets a <select>
    // with no <option> children - a no-op - after which the browser selects
    // the FIRST option as the children are inserted. Effects, unlike
    // RenderEffects created during build, run after the render pass, so by
    // the time this body executes the element has its options. It re-runs on
    // mount (the NodeRef is a signal) and on every later `slot()` change.
    let bot_select = NodeRef::<leptos::html::Select>::new();
    Effect::new(move |_| {
        // NodeRef::get() is Option - never unwrap it (docs/CODING.md:63-65).
        // None on SSR and before mount.
        let Some(el) = bot_select.get() else {
            return;
        };
        el.set_value(&bot_name_value());
    });

    view! {
        <div class="form-field opponent-slot">
            <label class="form-label">{label.clone()}</label>
            <div
                class="form-control slot-modes"
                role="radiogroup"
                aria-label=format!("{label} type")
            >
                <label>
                    <input
                        type="radio"
                        name=radio_group.clone()
                        prop:checked=move || mode() == SlotMode::Player
                        on:change=move |_| set_mode(SlotMode::Player)
                    />
                    " Player"
                </label>
                <label>
                    <input
                        type="radio"
                        name=radio_group.clone()
                        prop:checked=move || mode() == SlotMode::Email
                        on:change=move |_| set_mode(SlotMode::Email)
                    />
                    " Email"
                </label>
                <label>
                    <input
                        type="radio"
                        name=radio_group.clone()
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
                        aria-label=format!("Search players for {label}")
                        prop:value=slot_query
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            set.run(OpponentSlot::Player {
                                query: val.clone(),
                                selected: None,
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
                        let tk = taken.get();
                        search_results()
                            .into_iter()
                            .filter(|r| !tk.contains(&r.user_id))
                            .map(|r| {
                                let id = r.user_id;
                                let name = r.name.clone();
                                let text = r.name.clone();
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
                                            {text}
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
                                    let tk = taken.get();
                                    sugs.iter()
                                        .filter(|s| !tk.contains(&s.user_id))
                                        .map(|s| {
                                            let id = s.user_id;
                                            let name = s.name.clone();
                                            let text = s.name.clone();
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
                                                    {text}
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
                            set.run(OpponentSlot::Email(val));
                        }
                    />
                </div>
            </Show>
            <Show when=move || mode() == SlotMode::Bot>
                <div class="form-control">
                    {move || bot_name_options().map(|names| view! {
                        <select
                            aria-label="Bot difficulty"
                            node_ref=bot_select
                            prop:value=bot_name_value
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                if let OpponentSlot::Bot { name, .. } = get.get_untracked() {
                                    set.run(OpponentSlot::Bot { name, bot_name: val });
                                }
                            }
                        >
                            {names
                                .into_iter()
                                .map(|n| {
                                    let text = n.clone();
                                    view! {
                                        <option value=n>{text}</option>
                                    }
                                })
                                .collect_view()}
                        </select>
                    })}
                </div>
            </Show>
        </div>
    }
}
