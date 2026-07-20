use brdgme_game::command::Spec;
use minijinja::{Environment, context};
use serde::Serialize;

const SYSTEM_TEMPLATE: &str = include_str!("../system_prompt.md");
const USER_TEMPLATE: &str = include_str!("../user_prompt.md");

#[derive(Debug, Serialize)]
pub struct PlayerInfo {
    pub name: String,
    pub colour: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct FailedCommand {
    pub command: String,
    pub error: String,
}

#[derive(Debug)]
pub struct SystemContext {
    pub game_rules: String,
    pub include_basic_strategy: bool,
    pub basic_strategy: String,
    pub include_advanced_strategy: bool,
    pub advanced_strategy: String,
    pub data_docs: String,
}

#[derive(Debug)]
pub struct UserContext {
    pub my_name: String,
    pub my_colour: String,
    pub players: Vec<PlayerInfo>,
    pub pub_state_yaml: String,
    pub player_state_yaml: String,
    pub command_spec: String,
    pub recent_logs: Vec<String>,
    pub failed_commands: Vec<FailedCommand>,
}

/// Resolve `{{player N}}` references in brdgme markup to player names.
/// All other markup tags are passed through unchanged.
pub fn markup_resolve_players(markup: &str, names: &[String]) -> String {
    let mut result = markup.to_string();
    for (i, name) in names.iter().enumerate() {
        result = result.replace(&format!("{{{{player {i}}}}}"), name);
    }
    result
}

/// Serialise a command Spec to a YAML string.
///
/// serde_yaml 0.9 uses YAML native tags (`!Variant value`) for enum variants,
/// which differs from the mapping style documented in the system prompt
/// (`Variant: value`). Routing through serde_json::Value first produces the
/// mapping style we want, since JSON's object representation maps directly to
/// YAML mappings.
pub fn spec_to_yaml(spec: &Spec) -> String {
    let json_val = serde_json::to_value(spec).unwrap_or_default();
    serde_yaml::to_string(&json_val).unwrap_or_default()
}

pub fn render_system(ctx: &SystemContext) -> Result<String, minijinja::Error> {
    let mut env = Environment::new();
    env.add_template("prompt", SYSTEM_TEMPLATE)?;
    let tmpl = env.get_template("prompt")?;
    tmpl.render(context! {
        game_rules => &ctx.game_rules,
        include_basic_strategy => ctx.include_basic_strategy,
        basic_strategy => &ctx.basic_strategy,
        include_advanced_strategy => ctx.include_advanced_strategy,
        advanced_strategy => &ctx.advanced_strategy,
        data_docs => &ctx.data_docs,
    })
}

pub fn render_user(ctx: &UserContext) -> Result<String, minijinja::Error> {
    let mut env = Environment::new();
    env.add_template("prompt", USER_TEMPLATE)?;
    let tmpl = env.get_template("prompt")?;
    tmpl.render(context! {
        my_name => &ctx.my_name,
        my_colour => &ctx.my_colour,
        players => &ctx.players,
        pub_state_yaml => &ctx.pub_state_yaml,
        player_state_yaml => &ctx.player_state_yaml,
        command_spec => &ctx.command_spec,
        recent_logs => &ctx.recent_logs,
        failed_commands => &ctx.failed_commands,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system_ctx() -> SystemContext {
        SystemContext {
            game_rules: "## Rules\n\nPlace tiles and buy shares.".to_string(),
            include_basic_strategy: true,
            basic_strategy: "Buy cheap chains early.".to_string(),
            include_advanced_strategy: false,
            advanced_strategy: "Track tile counts.".to_string(),
            data_docs: "board: the board grid".to_string(),
        }
    }

    fn user_ctx() -> UserContext {
        UserContext {
            my_name: "Alice".to_string(),
            my_colour: "#4caf50".to_string(),
            players: vec![
                PlayerInfo {
                    name: "Alice".to_string(),
                    colour: "#4caf50".to_string(),
                    score: 6000.0,
                },
                PlayerInfo {
                    name: "Bob".to_string(),
                    colour: "#f44336".to_string(),
                    score: 4500.0,
                },
            ],
            pub_state_yaml: "board: empty\nround: 1".to_string(),
            player_state_yaml: "hand:\n  - A\n  - K".to_string(),
            command_spec: "Token: done".to_string(),
            recent_logs: vec![
                "Alice placed {{b}}C4{{/b}}".to_string(),
                "Bob bought 2 Sackson".to_string(),
            ],
            failed_commands: vec![],
        }
    }

    #[test]
    fn render_system_includes_game_rules() {
        let output = render_system(&system_ctx()).unwrap();
        assert!(
            output.contains("Place tiles and buy shares"),
            "game rules missing"
        );
    }

    #[test]
    fn render_system_omits_rules_section_when_empty() {
        let mut ctx = system_ctx();
        ctx.game_rules = String::new();
        let output = render_system(&ctx).unwrap();
        assert!(
            !output.contains("Place tiles"),
            "rules shown when they should be absent"
        );
    }

    #[test]
    fn render_system_includes_basic_strategy_only_when_enabled() {
        let mut ctx = system_ctx();
        ctx.include_basic_strategy = true;
        let output = render_system(&ctx).unwrap();
        assert!(
            output.contains("Buy cheap chains early"),
            "basic strategy missing when enabled"
        );

        ctx.include_basic_strategy = false;
        let output = render_system(&ctx).unwrap();
        assert!(
            !output.contains("Buy cheap chains early"),
            "basic strategy shown when disabled"
        );
    }

    #[test]
    fn render_system_includes_advanced_strategy_only_when_enabled() {
        let mut ctx = system_ctx();
        ctx.include_advanced_strategy = true;
        let output = render_system(&ctx).unwrap();
        assert!(
            output.contains("Track tile counts"),
            "advanced strategy missing when enabled"
        );

        ctx.include_advanced_strategy = false;
        let output = render_system(&ctx).unwrap();
        assert!(
            !output.contains("Track tile counts"),
            "advanced strategy shown when disabled"
        );
    }

    #[test]
    fn render_system_includes_data_docs() {
        let output = render_system(&system_ctx()).unwrap();
        assert!(
            output.contains("board: the board grid"),
            "data docs missing"
        );
    }

    #[test]
    fn render_system_contains_command_parser_docs() {
        let output = render_system(&system_ctx()).unwrap();
        assert!(output.contains("OneOf"), "OneOf docs missing");
        assert!(output.contains("Token"), "Token docs missing");
    }

    #[test]
    fn render_user_renders_players_with_score_and_colour() {
        let output = render_user(&user_ctx()).unwrap();
        assert!(output.contains("Alice"), "player 1 name missing");
        assert!(output.contains("6000"), "player 1 score missing");
        assert!(output.contains("#4caf50"), "player 1 colour missing");
        assert!(output.contains("Bob"), "player 2 name missing");
        assert!(output.contains("4500"), "player 2 score missing");
        assert!(output.contains("#f44336"), "player 2 colour missing");
    }

    #[test]
    fn render_user_marks_self_in_player_list() {
        let output = render_user(&user_ctx()).unwrap();
        assert!(output.contains("Alice (you)"), "self marker missing");
        assert!(
            !output.contains("Bob (you)"),
            "Bob incorrectly marked as self"
        );
    }

    #[test]
    fn render_user_includes_state_in_yaml_fences() {
        let output = render_user(&user_ctx()).unwrap();
        assert!(
            output.contains("```yaml\nboard: empty\nround: 1"),
            "pub state not in yaml fence"
        );
        assert!(
            output.contains("```yaml\nhand:\n  - A\n  - K"),
            "player state not in yaml fence"
        );
    }

    #[test]
    fn render_user_includes_command_spec() {
        let output = render_user(&user_ctx()).unwrap();
        assert!(output.contains("Token: done"), "command spec missing");
    }

    #[test]
    fn render_user_renders_recent_logs() {
        let output = render_user(&user_ctx()).unwrap();
        assert!(
            output.contains("Alice placed {{b}}C4{{/b}}"),
            "log 1 missing"
        );
        assert!(output.contains("Bob bought 2 Sackson"), "log 2 missing");
    }

    #[test]
    fn render_user_omits_failed_commands_section_when_empty() {
        let output = render_user(&user_ctx()).unwrap();
        assert!(
            !output.contains("previously responded"),
            "failed commands section shown when there are none"
        );
    }

    #[test]
    fn render_user_renders_failed_commands_when_present() {
        let mut ctx = user_ctx();
        ctx.failed_commands = vec![
            FailedCommand {
                command: "buy 5 sackson".to_string(),
                error: "cannot buy more than 3 shares".to_string(),
            },
            FailedCommand {
                command: "buy 0 tower".to_string(),
                error: "minimum 1 share".to_string(),
            },
        ];
        let output = render_user(&ctx).unwrap();
        assert!(
            output.contains("previously responded"),
            "failed commands header missing"
        );
        assert!(output.contains("buy 5 sackson"), "failed command 1 missing");
        assert!(
            output.contains("cannot buy more than 3 shares"),
            "error 1 missing"
        );
        assert!(output.contains("buy 0 tower"), "failed command 2 missing");
        assert!(output.contains("minimum 1 share"), "error 2 missing");
    }

    #[test]
    fn render_user_ends_with_command_instruction() {
        let output = render_user(&user_ctx()).unwrap();
        assert!(
            output.contains("Please provide your command now."),
            "final instruction missing"
        );
    }

    #[test]
    fn markup_resolve_players_replaces_player_refs() {
        let names = vec!["Alice".to_string(), "Bob".to_string()];
        let markup = "{{player 0}} played a tile, then {{player 1}} responded";
        let out = markup_resolve_players(markup, &names);
        assert_eq!(out, "Alice played a tile, then Bob responded");
    }

    #[test]
    fn markup_resolve_players_leaves_other_tags_intact() {
        let names = vec!["Alice".to_string()];
        let markup = "{{b}}bold{{/b}} and {{fg rgb(255,0,0)}}red{{/fg}}";
        let out = markup_resolve_players(markup, &names);
        assert_eq!(out, markup);
    }

    #[test]
    fn spec_to_yaml_produces_expected_format() {
        let spec = Spec::OneOf(vec![
            Spec::Token("done".to_string()),
            Spec::Int {
                min: Some(1),
                max: Some(3),
            },
        ]);
        let yaml = spec_to_yaml(&spec);
        // Top-level key should be the variant name as a mapping key, not a YAML tag.
        assert!(
            yaml.contains("OneOf:"),
            "OneOf mapping key missing: {}",
            yaml
        );
        assert!(
            yaml.contains("Token: done"),
            "Token variant missing: {}",
            yaml
        );
        assert!(yaml.contains("min: 1"), "min missing: {}", yaml);
        assert!(yaml.contains("max: 3"), "max missing: {}", yaml);
        // Must NOT use YAML native tags.
        assert!(
            !yaml.contains('!'),
            "YAML tags found - wrong format: {}",
            yaml
        );
    }
}
