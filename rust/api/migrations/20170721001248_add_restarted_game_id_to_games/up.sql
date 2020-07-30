ALTER TABLE games
ADD COLUMN restarted_game_id UUID REFERENCES games (id);
