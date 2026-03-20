use crate::command::{Spec, Suggestion};
use crate::command::parser::Parser;

impl Spec {
    pub fn suggest(&self, input: &str, names: &[String]) -> Vec<Suggestion> {
        suggest_spec(self, input, names)
    }
}

fn suggest_spec(spec: &Spec, remaining: &str, names: &[String]) -> Vec<Suggestion> {
    match spec {
        Spec::Token(token) => {
            if token.to_lowercase().starts_with(&remaining.to_lowercase()) {
                vec![Suggestion { value: token.clone(), desc: None }]
            } else {
                vec![]
            }
        }
        Spec::Enum { values, .. } => {
            let lower = remaining.to_lowercase();
            values.iter()
                .filter(|v| v.to_lowercase().starts_with(&lower))
                .map(|v| Suggestion { value: v.clone(), desc: None })
                .collect()
        }
        Spec::OneOf(specs) => {
            specs.iter().flat_map(|s| suggest_spec(s, remaining, names)).collect()
        }
        Spec::Doc { desc, spec, .. } => {
            let suggs = suggest_spec(spec, remaining, names);
            let at_current_pos = suggs.iter().any(|s| {
                s.value.to_lowercase().starts_with(&remaining.to_lowercase())
            });
            if !at_current_pos {
                return suggs;
            }
            suggs.into_iter().map(|mut s| {
                if s.desc.is_none() {
                    s.desc = desc.clone();
                }
                s
            }).collect()
        }
        Spec::Chain(specs) => {
            let mut rem = remaining;
            for spec in specs {
                let suggs = suggest_spec(spec, rem, names);
                if !suggs.is_empty() {
                    return suggs;
                }
                match spec.parse(rem, names) {
                    Ok(out) => rem = out.remaining,
                    Err(_) => return vec![],
                }
            }
            vec![]
        }
        Spec::Space => vec![],
        Spec::Int { min, max } => {
            let start = min.unwrap_or(1);
            let end = max.map(|m| m.min(start + 4)).unwrap_or(start + 4);
            (start..=end)
                .map(|i| i.to_string())
                .filter(|s| s.starts_with(remaining))
                .map(|v| Suggestion { value: v, desc: None })
                .collect()
        }
        Spec::Player => {
            let lower = remaining.to_lowercase();
            names.iter()
                .filter(|n| n.to_lowercase().starts_with(&lower))
                .map(|n| Suggestion { value: n.clone(), desc: None })
                .collect()
        }
        Spec::Opt(spec) => suggest_spec(spec, remaining, names),
        Spec::Many { spec, .. } => suggest_spec(spec, remaining, names),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(ns: &[&str]) -> Vec<String> {
        ns.iter().map(|s| s.to_string()).collect()
    }

    fn vals(suggestions: &[Suggestion]) -> Vec<&str> {
        suggestions.iter().map(|s| s.value.as_str()).collect()
    }

    fn descs(suggestions: &[Suggestion]) -> Vec<Option<&str>> {
        suggestions.iter().map(|s| s.desc.as_deref()).collect()
    }

    // --- Token ---

    #[test]
    fn token_empty_input_suggests_full_token() {
        let s = Spec::Token("play".into()).suggest("", &[]);
        assert_eq!(vals(&s), vec!["play"]);
    }

    #[test]
    fn token_prefix_suggests_full_token() {
        let s = Spec::Token("play".into()).suggest("pl", &[]);
        assert_eq!(vals(&s), vec!["play"]);
    }

    #[test]
    fn token_exact_match_still_suggests() {
        // Full token typed but no space yet - still suggest so user can click/confirm.
        let s = Spec::Token("play".into()).suggest("play", &[]);
        assert_eq!(vals(&s), vec!["play"]);
    }

    #[test]
    fn token_no_match_returns_empty() {
        let s = Spec::Token("play".into()).suggest("buy", &[]);
        assert!(s.is_empty());
    }

    #[test]
    fn token_case_insensitive_prefix() {
        let s = Spec::Token("play".into()).suggest("PL", &[]);
        assert_eq!(vals(&s), vec!["play"]);
    }

    #[test]
    fn token_input_longer_than_token_no_match() {
        // "playx" cannot match token "play"
        let s = Spec::Token("play".into()).suggest("playx", &[]);
        assert!(s.is_empty());
    }

    // --- Enum ---

    #[test]
    fn enum_empty_input_returns_all_values() {
        let spec = Spec::Enum { values: vec!["buy".into(), "sell".into(), "play".into()], exact: false };
        let s = spec.suggest("", &[]);
        assert_eq!(vals(&s), vec!["buy", "sell", "play"]);
    }

    #[test]
    fn enum_prefix_filters_values() {
        let spec = Spec::Enum { values: vec!["buy".into(), "sell".into(), "play".into()], exact: false };
        let s = spec.suggest("p", &[]);
        assert_eq!(vals(&s), vec!["play"]);
    }

    #[test]
    fn enum_no_match_returns_empty() {
        let spec = Spec::Enum { values: vec!["buy".into(), "sell".into()], exact: false };
        let s = spec.suggest("x", &[]);
        assert!(s.is_empty());
    }

    #[test]
    fn enum_case_insensitive() {
        let spec = Spec::Enum { values: vec!["Sackson".into(), "Tower".into()], exact: false };
        let s = spec.suggest("sa", &[]);
        assert_eq!(vals(&s), vec!["Sackson"]);
    }

    #[test]
    fn enum_exact_same_prefix_behavior() {
        // exact flag affects parsing but not prefix-based suggestion generation
        let spec = Spec::Enum { values: vec!["one".into(), "other".into()], exact: true };
        let s = spec.suggest("o", &[]);
        assert_eq!(vals(&s), vec!["one", "other"]);
    }

    // --- OneOf ---

    #[test]
    fn one_of_empty_input_returns_all_branches() {
        let spec = Spec::OneOf(vec![
            Spec::Token("buy".into()),
            Spec::Token("sell".into()),
        ]);
        let s = spec.suggest("", &[]);
        assert_eq!(vals(&s), vec!["buy", "sell"]);
    }

    #[test]
    fn one_of_prefix_filters_to_matching_branch() {
        let spec = Spec::OneOf(vec![
            Spec::Token("buy".into()),
            Spec::Token("sell".into()),
        ]);
        let s = spec.suggest("s", &[]);
        assert_eq!(vals(&s), vec!["sell"]);
    }

    #[test]
    fn one_of_no_match_returns_empty() {
        let spec = Spec::OneOf(vec![
            Spec::Token("buy".into()),
            Spec::Token("sell".into()),
        ]);
        let s = spec.suggest("x", &[]);
        assert!(s.is_empty());
    }

    // --- Doc ---

    #[test]
    fn doc_empty_input_returns_name_and_desc() {
        let spec = Spec::Doc {
            name: "play".into(),
            desc: Some("play a tile to the board".into()),
            spec: Box::new(Spec::Token("play".into())),
        };
        let s = spec.suggest("", &[]);
        assert_eq!(vals(&s), vec!["play"]);
        assert_eq!(descs(&s), vec![Some("play a tile to the board")]);
    }

    #[test]
    fn doc_prefix_of_name_returns_name_and_desc() {
        let spec = Spec::Doc {
            name: "play".into(),
            desc: Some("play a tile".into()),
            spec: Box::new(Spec::Token("play".into())),
        };
        let s = spec.suggest("pl", &[]);
        assert_eq!(vals(&s), vec!["play"]);
        assert_eq!(descs(&s), vec![Some("play a tile")]);
    }

    #[test]
    fn doc_exact_name_still_suggests() {
        let spec = Spec::Doc {
            name: "play".into(),
            desc: Some("play a tile".into()),
            spec: Box::new(Spec::Token("play".into())),
        };
        let s = spec.suggest("play", &[]);
        assert_eq!(vals(&s), vec!["play"]);
    }

    #[test]
    fn doc_no_desc_returns_none() {
        let spec = Spec::Doc {
            name: "play".into(),
            desc: None,
            spec: Box::new(Spec::Token("play".into())),
        };
        let s = spec.suggest("", &[]);
        assert_eq!(vals(&s), vec!["play"]);
        assert_eq!(descs(&s), vec![None]);
    }

    #[test]
    fn doc_past_name_recurses_into_spec() {
        let spec = Spec::Doc {
            name: "play".into(),
            desc: Some("play a tile".into()),
            spec: Box::new(Spec::Chain(vec![
                Spec::Token("play".into()),
                Spec::Space,
                Spec::Enum { values: vec!["A1".into(), "B2".into()], exact: false },
            ])),
        };
        let s = spec.suggest("play ", &[]);
        assert_eq!(vals(&s), vec!["A1", "B2"]);
        // desc not attached - suggestions came from chain advancement, not the doc's position
        assert_eq!(descs(&s), vec![None, None]);
    }

    #[test]
    fn doc_no_match_for_name_returns_empty() {
        let spec = Spec::Doc {
            name: "play".into(),
            desc: Some("play a tile".into()),
            spec: Box::new(Spec::Token("play".into())),
        };
        let s = spec.suggest("buy", &[]);
        assert!(s.is_empty());
    }

    // --- Chain ---

    #[test]
    fn chain_empty_input_suggests_first_spec() {
        let spec = Spec::Chain(vec![
            Spec::Token("play".into()),
            Spec::Space,
            Spec::Token("tile".into()),
        ]);
        let s = spec.suggest("", &[]);
        assert_eq!(vals(&s), vec!["play"]);
    }

    #[test]
    fn chain_advances_after_successful_parse() {
        let spec = Spec::Chain(vec![
            Spec::Token("play".into()),
            Spec::Space,
            Spec::Enum { values: vec!["A1".into(), "B2".into()], exact: false },
        ]);
        let s = spec.suggest("play ", &[]);
        assert_eq!(vals(&s), vec!["A1", "B2"]);
    }

    #[test]
    fn chain_prefix_of_second_element() {
        let spec = Spec::Chain(vec![
            Spec::Token("play".into()),
            Spec::Space,
            Spec::Enum { values: vec!["A1".into(), "A2".into(), "B1".into()], exact: false },
        ]);
        let s = spec.suggest("play A", &[]);
        assert_eq!(vals(&s), vec!["A1", "A2"]);
    }

    #[test]
    fn chain_fully_consumed_returns_empty() {
        let spec = Spec::Chain(vec![Spec::Token("play".into())]);
        // Token parses "play" successfully, chain is exhausted
        let s = spec.suggest("play ", &[]);
        // "play " - Token("play") parses "play", remaining " ", Space would be next
        // but there is no next spec, so empty
        assert!(s.is_empty());
    }

    // --- Int ---

    #[test]
    fn int_empty_input_generates_range() {
        let spec = Spec::Int { min: Some(1), max: Some(3) };
        let s = spec.suggest("", &[]);
        assert_eq!(vals(&s), vec!["1", "2", "3"]);
    }

    #[test]
    fn int_caps_range_at_5_values() {
        let spec = Spec::Int { min: Some(1), max: Some(100) };
        let s = spec.suggest("", &[]);
        assert_eq!(s.len(), 5);
        assert_eq!(vals(&s), vec!["1", "2", "3", "4", "5"]);
    }

    #[test]
    fn int_no_min_defaults_to_1() {
        let spec = Spec::Int { min: None, max: Some(3) };
        let s = spec.suggest("", &[]);
        assert_eq!(vals(&s), vec!["1", "2", "3"]);
    }

    #[test]
    fn int_prefix_filters_values() {
        let spec = Spec::Int { min: Some(10), max: Some(19) };
        let s = spec.suggest("1", &[]);
        assert!(vals(&s).iter().all(|v| v.starts_with('1')));
    }

    #[test]
    fn int_no_prefix_match_returns_empty() {
        let spec = Spec::Int { min: Some(1), max: Some(5) };
        let s = spec.suggest("9", &[]);
        assert!(s.is_empty());
    }

    // --- Player ---

    #[test]
    fn player_empty_input_returns_all_names() {
        let s = Spec::Player.suggest("", &names(&["alice", "bob"]));
        assert_eq!(vals(&s), vec!["alice", "bob"]);
    }

    #[test]
    fn player_prefix_filters_names() {
        let s = Spec::Player.suggest("a", &names(&["alice", "bob", "alan"]));
        assert_eq!(vals(&s), vec!["alice", "alan"]);
    }

    #[test]
    fn player_case_insensitive() {
        let s = Spec::Player.suggest("AL", &names(&["alice", "bob"]));
        assert_eq!(vals(&s), vec!["alice"]);
    }

    #[test]
    fn player_no_match_returns_empty() {
        let s = Spec::Player.suggest("x", &names(&["alice", "bob"]));
        assert!(s.is_empty());
    }

    #[test]
    fn player_empty_names_returns_empty() {
        let s = Spec::Player.suggest("", &[]);
        assert!(s.is_empty());
    }

    // --- Space ---

    #[test]
    fn space_never_suggests() {
        assert!(Spec::Space.suggest("", &[]).is_empty());
        assert!(Spec::Space.suggest("x", &[]).is_empty());
        assert!(Spec::Space.suggest(" ", &[]).is_empty());
    }

    // --- Opt ---

    #[test]
    fn opt_passes_through_to_inner_spec() {
        let spec = Spec::Opt(Box::new(Spec::Token("maybe".into())));
        let s = spec.suggest("m", &[]);
        assert_eq!(vals(&s), vec!["maybe"]);
    }

    #[test]
    fn opt_empty_input_suggests_inner() {
        let spec = Spec::Opt(Box::new(Spec::Token("loud".into())));
        let s = spec.suggest("", &[]);
        assert_eq!(vals(&s), vec!["loud"]);
    }

    // --- Many ---

    #[test]
    fn many_suggests_from_item_spec() {
        let spec = Spec::Many {
            spec: Box::new(Spec::Enum { values: vec!["1".into(), "2".into(), "3".into()], exact: false }),
            min: None,
            max: None,
            delim: None,
        };
        let s = spec.suggest("", &[]);
        assert_eq!(vals(&s), vec!["1", "2", "3"]);
    }

    #[test]
    fn many_with_delimiter_known_limitation() {
        // Known limitation: after typing "1, " the delimiter is not consumed,
        // so prefix "1, " doesn't match any enum value starting with "1, ".
        let spec = Spec::Many {
            spec: Box::new(Spec::Enum { values: vec!["1".into(), "2".into()], exact: false }),
            min: None,
            max: None,
            delim: Some(Box::new(Spec::Token(", ".into()))),
        };
        // This correctly suggests nothing (limitation documented, not a crash)
        let s = spec.suggest("1, ", &[]);
        assert!(s.is_empty());
    }

    // --- Realistic game scenarios ---

    fn acquire_spec() -> Spec {
        Spec::OneOf(vec![
            Spec::Doc {
                name: "play".into(),
                desc: Some("play a tile to the board".into()),
                spec: Box::new(Spec::Chain(vec![
                    Spec::Token("play".into()),
                    Spec::Space,
                    Spec::Enum { values: vec!["A1".into(), "A2".into(), "B1".into()], exact: false },
                ])),
            },
            Spec::Doc {
                name: "buy".into(),
                desc: Some("buy shares".into()),
                spec: Box::new(Spec::Chain(vec![
                    Spec::Token("buy".into()),
                    Spec::Space,
                    Spec::Enum { values: vec!["Sackson".into(), "Tower".into()], exact: false },
                ])),
            },
        ])
    }

    #[test]
    fn game_empty_input_shows_all_commands_with_desc() {
        let s = acquire_spec().suggest("", &[]);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].value, "play");
        assert_eq!(s[0].desc.as_deref(), Some("play a tile to the board"));
        assert_eq!(s[1].value, "buy");
        assert_eq!(s[1].desc.as_deref(), Some("buy shares"));
    }

    #[test]
    fn game_prefix_p_filters_to_play() {
        let s = acquire_spec().suggest("p", &[]);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].value, "play");
        assert_eq!(s[0].desc.as_deref(), Some("play a tile to the board"));
    }

    #[test]
    fn game_prefix_b_filters_to_buy() {
        let s = acquire_spec().suggest("b", &[]);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].value, "buy");
    }

    #[test]
    fn game_full_command_name_still_suggests_itself() {
        let s = acquire_spec().suggest("play", &[]);
        assert_eq!(vals(&s), vec!["play"]);
    }

    #[test]
    fn game_after_play_space_shows_tile_enum() {
        let s = acquire_spec().suggest("play ", &[]);
        assert_eq!(vals(&s), vec!["A1", "A2", "B1"]);
    }

    #[test]
    fn game_tile_prefix_filters_enum() {
        let s = acquire_spec().suggest("play A", &[]);
        assert_eq!(vals(&s), vec!["A1", "A2"]);
    }

    #[test]
    fn game_after_buy_space_shows_corp_enum() {
        let s = acquire_spec().suggest("buy ", &[]);
        assert_eq!(vals(&s), vec!["Sackson", "Tower"]);
    }

    #[test]
    fn game_corp_prefix_filters() {
        let s = acquire_spec().suggest("buy S", &[]);
        assert_eq!(vals(&s), vec!["Sackson"]);
    }

    #[test]
    fn game_no_match_returns_empty() {
        let s = acquire_spec().suggest("x", &[]);
        assert!(s.is_empty());
    }

    fn attack_spec() -> Spec {
        Spec::Doc {
            name: "attack".into(),
            desc: Some("attack a player".into()),
            spec: Box::new(Spec::Chain(vec![
                Spec::Token("attack".into()),
                Spec::Space,
                Spec::Player,
            ])),
        }
    }

    #[test]
    fn player_cmd_empty_shows_command_name_with_desc() {
        let s = attack_spec().suggest("", &names(&["alice", "bob"]));
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].value, "attack");
        assert_eq!(s[0].desc.as_deref(), Some("attack a player"));
    }

    #[test]
    fn player_cmd_after_space_shows_all_players() {
        let s = attack_spec().suggest("attack ", &names(&["alice", "bob"]));
        assert_eq!(vals(&s), vec!["alice", "bob"]);
    }

    #[test]
    fn player_cmd_prefix_filters_players() {
        let s = attack_spec().suggest("attack a", &names(&["alice", "bob", "alan"]));
        assert_eq!(vals(&s), vec!["alice", "alan"]);
    }

    #[test]
    fn player_cmd_full_name_typed() {
        let s = attack_spec().suggest("attack alice", &names(&["alice", "bob"]));
        assert_eq!(vals(&s), vec!["alice"]);
    }

    // --- Space: multiple whitespace characters ---

    #[test]
    fn chain_multiple_spaces_between_tokens() {
        // Space parser consumes all consecutive whitespace, so two spaces work.
        let spec = Spec::Chain(vec![
            Spec::Token("play".into()),
            Spec::Space,
            Spec::Enum { values: vec!["A1".into(), "B2".into()], exact: false },
        ]);
        let s = spec.suggest("play  ", &[]);
        assert_eq!(vals(&s), vec!["A1", "B2"]);
    }

    #[test]
    fn chain_multiple_spaces_with_partial_enum_prefix() {
        let spec = Spec::Chain(vec![
            Spec::Token("play".into()),
            Spec::Space,
            Spec::Enum { values: vec!["A1".into(), "A2".into(), "B1".into()], exact: false },
        ]);
        let s = spec.suggest("play  A", &[]);
        assert_eq!(vals(&s), vec!["A1", "A2"]);
    }

    // --- Case-insensitive chain advancement ---

    #[test]
    fn chain_advances_past_uppercase_token() {
        // Token parser is case-insensitive (UniCase), so "PLAY" matches Token("play")
        // and the chain correctly advances to suggest the next spec.
        let spec = Spec::Chain(vec![
            Spec::Token("play".into()),
            Spec::Space,
            Spec::Enum { values: vec!["A1".into(), "B2".into()], exact: false },
        ]);
        let s = spec.suggest("PLAY ", &[]);
        assert_eq!(vals(&s), vec!["A1", "B2"]);
    }

    #[test]
    fn chain_advances_past_mixed_case_token_with_prefix() {
        let spec = Spec::Chain(vec![
            Spec::Token("play".into()),
            Spec::Space,
            Spec::Enum { values: vec!["A1".into(), "A2".into(), "B1".into()], exact: false },
        ]);
        let s = spec.suggest("Play A", &[]);
        assert_eq!(vals(&s), vec!["A1", "A2"]);
    }

    #[test]
    fn doc_with_chain_uppercase_command_still_suggests_enum() {
        let spec = Spec::Doc {
            name: "play".into(),
            desc: Some("play a tile".into()),
            spec: Box::new(Spec::Chain(vec![
                Spec::Token("play".into()),
                Spec::Space,
                Spec::Enum { values: vec!["A1".into(), "B2".into()], exact: false },
            ])),
        };
        // Uppercase command typed - chain advances, tile suggestions returned without desc
        let s = spec.suggest("PLAY ", &[]);
        assert_eq!(vals(&s), vec!["A1", "B2"]);
        assert_eq!(descs(&s), vec![None, None]);
    }

    // --- Nested Docs ---

    #[test]
    fn nested_doc_inner_desc_takes_priority() {
        // Inner doc attaches its desc first; outer doc won't override a non-None desc.
        let spec = Spec::Doc {
            name: "outer".into(),
            desc: Some("outer desc".into()),
            spec: Box::new(Spec::Doc {
                name: "inner".into(),
                desc: Some("inner desc".into()),
                spec: Box::new(Spec::Token("go".into())),
            }),
        };
        let s = spec.suggest("", &[]);
        assert_eq!(vals(&s), vec!["go"]);
        assert_eq!(descs(&s), vec![Some("inner desc")]);
    }

    #[test]
    fn nested_doc_outer_desc_used_when_inner_has_none() {
        // Inner doc has no desc, outer doc fills in its desc.
        let spec = Spec::Doc {
            name: "outer".into(),
            desc: Some("outer desc".into()),
            spec: Box::new(Spec::Doc {
                name: "inner".into(),
                desc: None,
                spec: Box::new(Spec::Token("go".into())),
            }),
        };
        let s = spec.suggest("", &[]);
        assert_eq!(vals(&s), vec!["go"]);
        assert_eq!(descs(&s), vec![Some("outer desc")]);
    }

    #[test]
    fn nested_doc_no_desc_past_chain_position() {
        // After typing past the command name, neither inner nor outer desc should appear.
        let spec = Spec::Doc {
            name: "outer".into(),
            desc: Some("outer desc".into()),
            spec: Box::new(Spec::Doc {
                name: "inner".into(),
                desc: Some("inner desc".into()),
                spec: Box::new(Spec::Chain(vec![
                    Spec::Token("go".into()),
                    Spec::Space,
                    Spec::Player,
                ])),
            }),
        };
        let s = spec.suggest("go ", &names(&["alice", "bob"]));
        assert_eq!(vals(&s), vec!["alice", "bob"]);
        assert_eq!(descs(&s), vec![None, None]);
    }

    // --- Chain containing OneOf of Docs ---

    fn move_spec() -> Spec {
        // Doc wrapping a chain whose second argument is a OneOf of direction Docs.
        Spec::Doc {
            name: "move".into(),
            desc: Some("move a unit".into()),
            spec: Box::new(Spec::Chain(vec![
                Spec::Token("move".into()),
                Spec::Space,
                Spec::OneOf(vec![
                    Spec::Doc {
                        name: "north".into(),
                        desc: Some("go north".into()),
                        spec: Box::new(Spec::Token("north".into())),
                    },
                    Spec::Doc {
                        name: "south".into(),
                        desc: Some("go south".into()),
                        spec: Box::new(Spec::Token("south".into())),
                    },
                ]),
            ])),
        }
    }

    #[test]
    fn chain_one_of_docs_empty_input_shows_outer_command() {
        let s = move_spec().suggest("", &[]);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].value, "move");
        assert_eq!(s[0].desc.as_deref(), Some("move a unit"));
    }

    #[test]
    fn chain_one_of_docs_after_space_shows_all_directions_with_desc() {
        let s = move_spec().suggest("move ", &[]);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].value, "north");
        assert_eq!(s[0].desc.as_deref(), Some("go north"));
        assert_eq!(s[1].value, "south");
        assert_eq!(s[1].desc.as_deref(), Some("go south"));
    }

    #[test]
    fn chain_one_of_docs_prefix_filters_to_matching_direction() {
        let s = move_spec().suggest("move n", &[]);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].value, "north");
        assert_eq!(s[0].desc.as_deref(), Some("go north"));
    }

    #[test]
    fn chain_one_of_docs_prefix_case_insensitive() {
        let s = move_spec().suggest("move N", &[]);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].value, "north");
        assert_eq!(s[0].desc.as_deref(), Some("go north"));
    }

    #[test]
    fn chain_one_of_docs_uppercase_command_still_advances() {
        // Chain advances case-insensitively past "MOVE", then shows directions.
        let s = move_spec().suggest("MOVE ", &[]);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].value, "north");
        assert_eq!(s[0].desc.as_deref(), Some("go north"));
    }

    #[test]
    fn chain_one_of_docs_multiple_spaces_before_direction() {
        let s = move_spec().suggest("move  ", &[]);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].value, "north");
        assert_eq!(s[0].desc.as_deref(), Some("go north"));
    }

    // --- Acquire buy phase (realistic nested spec from Chain3 + AfterSpace) ---
    //
    // Mirrors the exact to_spec() output of:
    //   Chain3(
    //     Doc("buy", "buy shares", Token("buy")),
    //     AfterSpace(Doc("#", "number of shares to buy", Int(1, 3))),
    //     AfterSpace(Doc("corp", "the corporation to buy shares in", Enum(CORPS, exact=false))),
    //   )
    // where AfterSpace::to_spec() = Chain([Space, inner]).

    fn acquire_buy_phase_spec() -> Spec {
        let corps = vec![
            "Worldwide".into(), "Sackson".into(), "Festival".into(),
            "Imperial".into(), "American".into(), "Continental".into(),
            "Tower".into(),
        ];
        Spec::OneOf(vec![
            Spec::Chain(vec![
                Spec::Doc {
                    name: "buy".into(),
                    desc: Some("buy shares".into()),
                    spec: Box::new(Spec::Token("buy".into())),
                },
                Spec::Chain(vec![
                    Spec::Space,
                    Spec::Doc {
                        name: "#".into(),
                        desc: Some("number of shares to buy".into()),
                        spec: Box::new(Spec::Int { min: Some(1), max: Some(3) }),
                    },
                ]),
                Spec::Chain(vec![
                    Spec::Space,
                    Spec::Doc {
                        name: "corp".into(),
                        desc: Some("the corporation to buy shares in".into()),
                        spec: Box::new(Spec::Enum { values: corps, exact: false }),
                    },
                ]),
            ]),
            Spec::Doc {
                name: "done".into(),
                desc: Some("finish buying shares and end your turn".into()),
                spec: Box::new(Spec::Token("done".into())),
            },
        ])
    }

    #[test]
    fn acquire_buy_empty_input_shows_buy_and_done() {
        let s = acquire_buy_phase_spec().suggest("", &[]);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].value, "buy");
        assert_eq!(s[0].desc.as_deref(), Some("buy shares"));
        assert_eq!(s[1].value, "done");
        assert_eq!(s[1].desc.as_deref(), Some("finish buying shares and end your turn"));
    }

    #[test]
    fn acquire_buy_typing_buy_shows_only_buy_not_done() {
        // "done" does not start with "buy", so it must be filtered out.
        // "whitespace" must never appear.
        let s = acquire_buy_phase_spec().suggest("buy", &[]);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].value, "buy");
        assert_eq!(s[0].desc.as_deref(), Some("buy shares"));
    }

    #[test]
    fn acquire_buy_after_buy_space_shows_int_options() {
        // After "buy " the chain advances through Doc+Space and suggests the Int range.
        let s = acquire_buy_phase_spec().suggest("buy ", &[]);
        assert_eq!(vals(&s), vec!["1", "2", "3"]);
        assert!(descs(&s).iter().all(|d| *d == Some("number of shares to buy")));
    }

    #[test]
    fn acquire_buy_after_count_space_shows_all_corps() {
        // After "buy 3 " the chain advances through both AfterSpace args and suggests all corps.
        let s = acquire_buy_phase_spec().suggest("buy 3 ", &[]);
        assert_eq!(
            vals(&s),
            vec!["Worldwide", "Sackson", "Festival", "Imperial", "American", "Continental", "Tower"]
        );
        assert!(descs(&s).iter().all(|d| *d == Some("the corporation to buy shares in")));
    }

    #[test]
    fn acquire_buy_corp_prefix_filters_to_festival() {
        // "fe" uniquely matches Festival (case-insensitive prefix).
        let s = acquire_buy_phase_spec().suggest("buy 3 fe", &[]);
        assert_eq!(vals(&s), vec!["Festival"]);
        assert_eq!(descs(&s), vec![Some("the corporation to buy shares in")]);
    }

    #[test]
    fn acquire_buy_corp_full_name_still_suggests() {
        // Full name typed - Enum::partial allows unique prefix matches (exact=false),
        // but the suggestion still returns the full name so the user sees it is valid.
        let s = acquire_buy_phase_spec().suggest("buy 3 Festival", &[]);
        assert_eq!(vals(&s), vec!["Festival"]);
    }

    #[test]
    fn acquire_buy_done_prefix_shows_only_done() {
        let s = acquire_buy_phase_spec().suggest("d", &[]);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].value, "done");
    }
}
