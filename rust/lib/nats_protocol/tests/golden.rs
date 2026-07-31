//! Golden-fixture wire-format test for the shared NATS protocol (R-14).
//!
//! Pins the exact JSON encoding of `BotTurnEvent` and `BotCommandEvent`
//! (field names, field order, UUID string form) so that the bot and web
//! consumers can never silently drift on the wire. Also asserts the shared
//! constants and the delivery-invariant relationships that both sides rely on.

use brdgme_nats::{
    ACK_WAIT, BotCommandEvent, BotTurnEvent, CONSUMER_COMMAND, CONSUMER_TURN, MAX_DELIVER,
    MAX_TURN_ATTEMPTS, STREAM_NAME, SUBJECT_COMMAND, SUBJECT_TURN,
};
use uuid::Uuid;

const GAME_ID: &str = "01234567-89ab-cdef-0123-456789abcdef";

const TURN_JSON: &str = r#"{"game_id":"01234567-89ab-cdef-0123-456789abcdef","player_position":2,"bot_name":"acquire-1","attempt":1}"#;
const COMMAND_JSON: &str = r#"{"game_id":"01234567-89ab-cdef-0123-456789abcdef","player_position":2,"command":"play tile A1","attempt":1}"#;

fn turn_event() -> BotTurnEvent {
    BotTurnEvent {
        game_id: Uuid::parse_str(GAME_ID).unwrap(),
        player_position: 2,
        bot_name: "acquire-1".to_string(),
        attempt: 1,
    }
}

fn command_event() -> BotCommandEvent {
    BotCommandEvent {
        game_id: Uuid::parse_str(GAME_ID).unwrap(),
        player_position: 2,
        command: "play tile A1".to_string(),
        attempt: 1,
    }
}

#[test]
fn bot_turn_event_serializes_to_golden_fixture() {
    let json = serde_json::to_string(&turn_event()).unwrap();
    assert_eq!(json, TURN_JSON, "BotTurnEvent wire format drifted");
}

#[test]
fn bot_command_event_serializes_to_golden_fixture() {
    let json = serde_json::to_string(&command_event()).unwrap();
    assert_eq!(json, COMMAND_JSON, "BotCommandEvent wire format drifted");
}

#[test]
fn bot_turn_event_deserializes_from_golden_fixture() {
    let event: BotTurnEvent = serde_json::from_str(TURN_JSON).unwrap();
    assert_eq!(event.game_id, Uuid::parse_str(GAME_ID).unwrap());
    assert_eq!(event.player_position, 2);
    assert_eq!(event.bot_name, "acquire-1");
    assert_eq!(event.attempt, 1);
}

#[test]
fn bot_command_event_deserializes_from_golden_fixture() {
    let event: BotCommandEvent = serde_json::from_str(COMMAND_JSON).unwrap();
    assert_eq!(event.game_id, Uuid::parse_str(GAME_ID).unwrap());
    assert_eq!(event.player_position, 2);
    assert_eq!(event.command, "play tile A1");
    assert_eq!(event.attempt, 1);
}

#[test]
fn round_trip_preserves_every_field() {
    let turn = turn_event();
    let turn_rt: BotTurnEvent =
        serde_json::from_str(&serde_json::to_string(&turn).unwrap()).unwrap();
    assert_eq!(turn_rt.game_id, turn.game_id);
    assert_eq!(turn_rt.player_position, turn.player_position);
    assert_eq!(turn_rt.bot_name, turn.bot_name);
    assert_eq!(turn_rt.attempt, turn.attempt);

    let command = command_event();
    let command_rt: BotCommandEvent =
        serde_json::from_str(&serde_json::to_string(&command).unwrap()).unwrap();
    assert_eq!(command_rt.game_id, command.game_id);
    assert_eq!(command_rt.player_position, command.player_position);
    assert_eq!(command_rt.command, command.command);
    assert_eq!(command_rt.attempt, command.attempt);
}

#[test]
fn uuid_serializes_as_hyphenated_lowercase_string() {
    let value = serde_json::to_value(turn_event()).unwrap();
    assert_eq!(value["game_id"], serde_json::json!(GAME_ID));
    assert!(value["game_id"].is_string());
}

#[test]
fn shared_constants_match_wire_values() {
    assert_eq!(STREAM_NAME, "BOT");
    assert_eq!(SUBJECT_TURN, "bot.turn");
    assert_eq!(SUBJECT_COMMAND, "bot.command");
    assert_eq!(CONSUMER_TURN, "bot-turn");
    assert_eq!(CONSUMER_COMMAND, "bot-command");
    assert_eq!(MAX_TURN_ATTEMPTS, 3);
    assert_eq!(MAX_DELIVER, 3);
}

#[test]
fn delivery_invariants_hold() {
    // JetStream must redeliver at least as many times as the app-level
    // turn-retry ceiling, and the ack window must outlast a long turn.
    assert!(i64::from(MAX_TURN_ATTEMPTS) <= MAX_DELIVER);
    assert_eq!(ACK_WAIT, std::time::Duration::from_secs(5 * 60));
}
