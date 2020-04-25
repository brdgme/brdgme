CREATE OR REPLACE FUNCTION update_is_turn_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.is_turn_at = now() AT TIME ZONE 'utc';
    RETURN NEW;	
END;
$$ language 'plpgsql';

CREATE TRIGGER update_is_turn_at
BEFORE UPDATE
ON game_players
FOR EACH ROW
WHEN (OLD.is_turn = FALSE AND NEW.is_turn = TRUE)
EXECUTE PROCEDURE update_is_turn_at();

CREATE OR REPLACE FUNCTION update_last_turn_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.last_turn_at = now() AT TIME ZONE 'utc';
    RETURN NEW;	
END;
$$ language 'plpgsql';

CREATE TRIGGER update_last_turn_at
BEFORE UPDATE
ON game_players
FOR EACH ROW
WHEN (OLD.is_turn = TRUE AND NEW.is_turn = FALSE)
EXECUTE PROCEDURE update_last_turn_at();