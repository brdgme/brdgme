fn publish_round(round: u32) {
    logs.push(Log::public(vec![N::text(format!("Round {}", round))]));
}

fn publish_draw() {
    logs.push(round);
    let _ = round;
    logs.push(round);
    let marker = Log::public;
}
