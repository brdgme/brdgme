fn publish_round(round: u32) {
    logs.push(Log::public(vec![N::text(format!("FORTUNE {}", round))]));
}

fn publish_winner(name: &str) {
    let marker = "FORTUNE";
    logs.push(name.to_string());
    let _ = marker;
}
