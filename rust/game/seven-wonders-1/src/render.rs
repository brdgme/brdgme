use brdgme_color::NamedColor;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::{CardEffect, CardKind, PlayerState, PubState};

fn kind_color(kind: CardKind) -> NamedColor {
    match kind {
        CardKind::Raw => NamedColor::Green,
        CardKind::Manufactured => NamedColor::Cyan,
        CardKind::Civilian => NamedColor::Blue,
        CardKind::Scientific => NamedColor::Purple,
        CardKind::Commercial => NamedColor::Yellow,
        CardKind::Military => NamedColor::Red,
        CardKind::Guild => NamedColor::Orange,
        CardKind::Wonder => NamedColor::Grey,
    }
}

fn render_card_name(name: &str, kind: CardKind) -> N {
    N::Fg(kind_color(kind).into(), vec![N::text(name.to_string())])
}

fn effect_desc(effect: &CardEffect) -> String {
    match effect {
        CardEffect::VP { vp } => format!("{}vp", vp),
        CardEffect::Military { strength } => format!("{}str", strength),
        CardEffect::Science { fields } => fields
            .iter()
            .map(|f| format!("{:?}", f))
            .collect::<Vec<_>>()
            .join("/"),
        CardEffect::Good { .. } => "resource".to_string(),
        CardEffect::Tavern => "5 coins".to_string(),
        CardEffect::Trade { .. } => "trade".to_string(),
        CardEffect::Bonus { vp, coins, .. } => match (*vp, *coins) {
            (0, 0) => "bonus".to_string(),
            (v, 0) => format!("{}vp each", v),
            (0, c) => format!("{} coins each", c),
            (v, c) => format!("{}vp/{} coins each", v, c),
        },
        CardEffect::Multi { .. } => "multi".to_string(),
        CardEffect::FreeBuild { .. } => "free build".to_string(),
        CardEffect::DrawDiscard { .. } => "draw discard".to_string(),
        CardEffect::MimicGuild => "mimic guild".to_string(),
        CardEffect::PlayFinalCard => "play final card".to_string(),
    }
}

fn render_player_tableau(state: &PubState, player: usize) -> N {
    let cards = &state.cards[player];
    if cards.is_empty() {
        return N::text("(no cards)");
    }
    let mut rows: Vec<Row> = vec![];
    for card in cards {
        rows.push(vec![
            (A::Left, vec![render_card_name(&card.name, card.kind)]),
            (
                A::Left,
                vec![N::text(format!("({})", effect_desc(&card.effect)))],
            ),
        ]);
    }
    table_with_gap(&rows, 1)
}

fn render_player_summary(state: &PubState) -> N {
    let mut rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Player")])]),
        (A::Left, vec![N::Bold(vec![N::text("City")])]),
        (A::Center, vec![N::Bold(vec![N::text("Coins")])]),
        (A::Center, vec![N::Bold(vec![N::text("VP")])]),
        (A::Center, vec![N::Bold(vec![N::text("Defeat")])]),
        (A::Center, vec![N::Bold(vec![N::text("Wonder")])]),
        (A::Center, vec![N::Bold(vec![N::text("Cards")])]),
        (A::Center, vec![N::Bold(vec![N::text("Hand")])]),
    ]];

    for p in 0..state.players {
        let city = &state.cities[p];
        let stages_built = state.cards[p]
            .iter()
            .filter(|c| c.kind == CardKind::Wonder)
            .count();
        let total_stages = city.wonder_stages.len();
        let tableau_count = state.cards[p]
            .iter()
            .filter(|c| c.kind != CardKind::Wonder)
            .count();

        rows.push(vec![
            (A::Left, vec![N::Player(p)]),
            (A::Left, vec![N::text(city.name.clone())]),
            (A::Center, vec![N::text(format!("{}", state.coins[p]))]),
            (
                A::Center,
                vec![N::text(format!("{}", state.victory_tokens[p]))],
            ),
            (
                A::Center,
                vec![N::text(format!("{}", state.defeat_tokens[p]))],
            ),
            (
                A::Center,
                vec![N::text(format!("{}/{}", stages_built, total_stages))],
            ),
            (A::Center, vec![N::text(format!("{}", tableau_count))]),
            (A::Center, vec![N::text(format!("{}", state.hand_sizes[p]))]),
        ]);
    }
    table_with_gap(&rows, 2)
}

fn render_pending(state: &PubState) -> Vec<N> {
    if state.finished {
        return vec![N::Bold(vec![N::text("Game over")])];
    }
    if let Some(p) = state.to_resolve_player {
        return vec![
            N::Bold(vec![N::text("Waiting for ")]),
            N::Player(p),
            N::Bold(vec![N::text(" to take from discard")]),
        ];
    }
    let waiting: Vec<usize> = (0..state.players)
        .filter(|&p| !state.actions_chosen[p] && state.hand_sizes[p] > 0)
        .collect();
    if waiting.is_empty() {
        return vec![];
    }
    let mut nodes = vec![N::Bold(vec![N::text("Waiting for: ")])];
    for (i, &p) in waiting.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text(", "));
        }
        nodes.push(N::Player(p));
    }
    nodes
}

fn render_game(state: &PubState, viewer: Option<usize>, hand: Option<&[crate::Card]>) -> Vec<N> {
    let mut rows: Vec<Row> = vec![];

    rows.push(vec![(
        A::Center,
        vec![
            N::Bold(vec![N::text("Age ")]),
            N::text(format!("{}", state.round)),
            N::text("    "),
            N::Bold(vec![N::text("Discard: ")]),
            N::text(format!("{}", state.discard_count)),
        ],
    )]);
    rows.push(vec![]);

    let pending = render_pending(state);
    if !pending.is_empty() {
        rows.push(vec![(A::Center, pending)]);
        rows.push(vec![]);
    }

    rows.push(vec![(A::Center, vec![render_player_summary(state)])]);
    rows.push(vec![]);

    if let Some(hand) = hand
        && !hand.is_empty()
    {
        rows.push(vec![(
            A::Center,
            vec![N::Bold(vec![N::text(format!(
                "Your hand ({} cards)",
                hand.len()
            ))])],
        )]);
        let mut hand_rows: Vec<Row> = vec![];
        for (i, card) in hand.iter().enumerate() {
            hand_rows.push(vec![
                (
                    A::Right,
                    vec![N::Bold(vec![N::text(format!("{}:", i + 1))])],
                ),
                (A::Left, vec![render_card_name(&card.name, card.kind)]),
                (
                    A::Left,
                    vec![N::text(format!("({})", effect_desc(&card.effect)))],
                ),
            ]);
        }
        rows.push(vec![(A::Center, vec![table_with_gap(&hand_rows, 1)])]);
        rows.push(vec![]);
    }

    let start = viewer.unwrap_or(0);
    for i in 0..state.players {
        let p = (start + i) % state.players;
        rows.push(vec![(
            A::Center,
            vec![
                N::Bold(vec![N::Player(p)]),
                N::text(format!(" - {}", state.cities[p].name)),
            ],
        )]);
        rows.push(vec![(A::Center, vec![render_player_tableau(state, p)])]);
        rows.push(vec![]);
    }

    vec![N::Table(rows)]
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render_game(self, None, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render_game(&self.public, Some(self.player), Some(&self.hand))
    }
}
