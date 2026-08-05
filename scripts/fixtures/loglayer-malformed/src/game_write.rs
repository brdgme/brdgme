fn publish_round(round: u32) {
    logs.push(Log::public(vec![N::text(format!("Round {}", round))]));
}

fn publish_winner(name: &str) {
    logs.push(Log::public(vec![N::text(name.to_string())]));
}
