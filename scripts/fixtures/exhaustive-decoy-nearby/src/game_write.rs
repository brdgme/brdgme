fn resolve_player_count(count: Option<u32>) -> u32 {
    match count {
        Some(n) => n,
        None => 2,
    }
}

fn first_card(player: &Player) -> &Card {
    match &player.hand[..] {
        [] => &EMPTY_CARD,
        [head, ..] => head,
    }
}
