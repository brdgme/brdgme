// Log::public(vec![N::text("The game is over")]);
fn publish_winner(name: &str) {
    logs.push(name.to_string());
}
