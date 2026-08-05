fn wrapper(name: &str) {
    logs.push(Log::public(vec![N::text(name.to_string())]));
}

fn caller() {
    logs.push(Log::public(vec![N::text("upper".to_string())]));
}
