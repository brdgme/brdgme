ALTER TABLE game_players
  ADD CONSTRAINT chk_left_at_requires_elimination_or_bot
  CHECK (left_at IS NULL OR is_eliminated OR game_bot_id IS NOT NULL);
