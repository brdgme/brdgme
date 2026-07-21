use leptos::prelude::*;
use uuid::Uuid;

#[derive(Copy, Clone, Debug)]
pub struct WebSocketTrigger {
    pub last_update: ReadSignal<u64>,
    pub set_last_update: WriteSignal<u64>,
}

#[derive(Copy, Clone, Debug)]
pub struct ProposalUpdate(pub RwSignal<Option<(Uuid, u64)>>);

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
        DummyEncoder, ReconnectLimit, UseWebSocketOptions, UseWebSocketReturn, use_document,
        use_event_listener, use_websocket_with_options,
    };

    let trigger = expect_context::<WebSocketTrigger>();
    let game_update = expect_context::<RwSignal<Option<(Uuid, u64)>>>();
    let proposal_update = expect_context::<ProposalUpdate>().0;

    let UseWebSocketReturn {
        ready_state: _,
        open,
        ..
    } = use_websocket_with_options::<String, String, FromToStringCodec, (), DummyEncoder>(
        "/ws",
        UseWebSocketOptions::default()
            .reconnect_limit(ReconnectLimit::Infinite)
            .on_message_raw(move |text: &str| {
                if let Ok(signal) = serde_json::from_str::<GameUpdateSignal>(text) {
                    trigger.set_last_update.update(|n| *n += 1);
                    bump_game_update(game_update, signal.game_id);
                } else if let Ok(signal) = serde_json::from_str::<ProposalUpdateSignal>(text) {
                    trigger.set_last_update.update(|n| *n += 1);
                    bump_proposal_update(proposal_update, signal.proposal_id);
                }
            }),
    );

    let open_vis = open.clone();
    use_event_listener(use_document(), visibilitychange, move |_| {
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
