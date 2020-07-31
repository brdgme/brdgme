CREATE OR REPLACE FUNCTION update_finished_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.finished_at = now() AT TIME ZONE 'utc';
    RETURN NEW;	
END;
$$ language 'plpgsql';

CREATE TRIGGER update_finished_at
BEFORE UPDATE
ON games
FOR EACH ROW
WHEN (OLD.is_finished = FALSE AND NEW.is_finished = TRUE)
EXECUTE PROCEDURE update_finished_at();
