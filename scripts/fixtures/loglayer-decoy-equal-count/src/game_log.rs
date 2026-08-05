fn publish_winner(name: &str) {
    let marker = Log::public;
    logs.push(name.to_string());
    let _ = marker;
}

fn publish_marker(name: &str) {
    logs.push(name.to_string());
    logs.push(Log::public(vec![N::text(name.to_string())]));
}
