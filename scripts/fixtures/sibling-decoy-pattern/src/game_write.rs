fn update_game_command_success() {
    apply_left_at_guard(true);
}

fn undo_game() {
    apply_left_at_guard(false);
}
