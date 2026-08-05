fn resolve_player_count(count: Option<u32>) -> u32 {
    match count {
        Some(n) => n,
        _ => 2,
    }
}

fn classify(x: i32) -> &'static str {
    match x {
        0 => "zero",
        _=> "other",
    }
}

fn describe(v: Option<i32>) -> String {
    match v {
        Some(n) if n > 0 => format!("pos {n}"),
        _ if v.is_none() => "none".into(),
        _ @ None => "none".into(),
        _ => "other".into(),
    }
}
