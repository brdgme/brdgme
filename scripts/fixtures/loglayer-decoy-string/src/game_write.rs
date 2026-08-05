fn publish_winner(name: &str) {
    let marker = "Log::public(vec![])";
    logs.push(name.to_string());
    let _ = marker;
}
