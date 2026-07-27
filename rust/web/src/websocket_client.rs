use leptos::prelude::*;
use uuid::Uuid;

#[derive(Copy, Clone, Debug)]
pub struct WebSocketTrigger {
    pub last_update: ReadSignal<u64>,
    pub set_last_update: WriteSignal<u64>,
}

#[derive(Copy, Clone, Debug)]
pub struct ProposalUpdate(pub RwSignal<Option<(Uuid, u64)>>);

#[derive(Copy, Clone, Debug)]
pub struct PublicEventsUrl(pub RwSignal<Option<String>>);

/// Bumps the game-changed context to a fresh (game_id, seq) pair, deriving
/// seq from the current context value (prev + 1) rather than a separate
/// counter - a second independent counter could reproduce a seq already seen
/// for that game, and the PartialEq-deduping memos would silently drop the
/// refetch. Used both by the WS message handler and by post-action success
/// effects so an own action refetches even if the WS is down. When the WS is
/// up this deliberately causes one redundant refetch (local bump + server
/// signal) - accepted on purpose, since gating the local bump on WS
/// ready_state would re-open a half-open-socket window where a player's own
/// move doesn't render.
pub fn bump_game_update(game_update: RwSignal<Option<(Uuid, u64)>>, game_id: Uuid) {
    game_update.update(|v| {
        let next = v.map(|(_, s)| s + 1).unwrap_or(1);
        *v = Some((game_id, next));
    });
}

pub fn bump_proposal_update(proposal_update: RwSignal<Option<(Uuid, u64)>>, proposal_id: Uuid) {
    proposal_update.update(|v| {
        let next = v.map(|(_, s)| s + 1).unwrap_or(1);
        *v = Some((proposal_id, next));
    });
}

#[cfg(feature = "hydrate")]
pub fn use_websocket() {
    use crate::websocket::{GameUpdateSignal, ProposalUpdateSignal};
    use codee::string::FromToStringCodec;
    use leptos::ev::visibilitychange;
    use leptos_use::{
        ReconnectLimit, UseEventSourceMessage, UseEventSourceOnEventReturn, UseEventSourceOptions,
        UseEventSourceReturn, use_document, use_event_listener, use_event_source_with_options,
    };

    let trigger = expect_context::<WebSocketTrigger>();
    let game_update = expect_context::<RwSignal<Option<(Uuid, u64)>>>();
    let proposal_update = expect_context::<ProposalUpdate>().0;

    let on_event = move |e: &web_sys::Event| {
        let event_type = e.type_();
        if event_type == "game" || event_type == "proposal" {
            if let Ok(msg) = UseEventSourceMessage::<String, FromToStringCodec>::try_from(e.clone())
            {
                if event_type == "game" {
                    if let Ok(signal) = serde_json::from_str::<GameUpdateSignal>(&msg.data) {
                        trigger.set_last_update.update(|n| *n += 1);
                        bump_game_update(game_update, signal.game_id);
                    }
                } else if let Ok(signal) = serde_json::from_str::<ProposalUpdateSignal>(&msg.data) {
                    trigger.set_last_update.update(|n| *n += 1);
                    bump_proposal_update(proposal_update, signal.proposal_id);
                }
            }
        }
        UseEventSourceOnEventReturn::ProcessMessage
    };

    let UseEventSourceReturn { open, .. } =
        use_event_source_with_options::<String, FromToStringCodec>(
            "/events",
            UseEventSourceOptions::default()
                .reconnect_limit(ReconnectLimit::Infinite)
                .named_events(vec!["game".to_string(), "proposal".to_string()])
                .on_event(on_event),
        );

    let open_vis = open.clone();
    let _ = use_event_listener(use_document(), visibilitychange, move |_| {
        let doc = web_sys::window()
            .and_then(|w| w.document())
            .expect("no document");
        if doc.visibility_state() == web_sys::VisibilityState::Visible {
            open_vis();
            trigger.set_last_update.update(|n| *n += 1);
        }
    });

    window_event_listener(leptos::ev::online, move |_| {
        open();
        trigger.set_last_update.update(|n| *n += 1);
    });
}

#[cfg(not(feature = "hydrate"))]
pub fn use_websocket() {}

#[cfg(feature = "hydrate")]
pub fn use_public_events() {
    use crate::websocket::GameUpdateSignal;
    use codee::string::FromToStringCodec;
    use leptos_use::{
        ReconnectLimit, UseEventSourceMessage, UseEventSourceOnEventReturn, UseEventSourceOptions,
        use_event_source_with_options,
    };

    let trigger = expect_context::<WebSocketTrigger>();
    let game_update = expect_context::<RwSignal<Option<(Uuid, u64)>>>();
    let url_signal = expect_context::<PublicEventsUrl>().0;

    let url: Signal<String> = Signal::derive(move || url_signal.get().unwrap_or_default());

    let on_event = move |e: &web_sys::Event| {
        if e.type_() == "game" {
            if let Ok(msg) = UseEventSourceMessage::<String, FromToStringCodec>::try_from(e.clone())
            {
                if let Ok(signal) = serde_json::from_str::<GameUpdateSignal>(&msg.data) {
                    trigger.set_last_update.update(|n| *n += 1);
                    bump_game_update(game_update, signal.game_id);
                }
            }
        }
        UseEventSourceOnEventReturn::ProcessMessage
    };

    let _ = use_event_source_with_options::<String, FromToStringCodec>(
        url,
        UseEventSourceOptions::default()
            .reconnect_limit(ReconnectLimit::Infinite)
            .named_events(vec!["game".to_string()])
            .on_event(on_event),
    );
}

#[cfg(not(feature = "hydrate"))]
pub fn use_public_events() {}

#[cfg(feature = "hydrate")]
#[component]
pub fn PublicEventsWatcher() -> impl IntoView {
    use leptos_router::hooks::use_location;

    let url_signal = expect_context::<PublicEventsUrl>().0;
    let location = use_location();

    Effect::new(move |_| {
        let path = location.pathname.get();
        let game_url = path
            .strip_prefix("/games/")
            .filter(|id| uuid::Uuid::parse_str(id).is_ok())
            .map(|id| format!("/events/public?topic=game:{id}"));
        url_signal.set(game_url);
    });

    view! {}
}

#[cfg(not(feature = "hydrate"))]
#[component]
pub fn PublicEventsWatcher() -> impl IntoView {}
