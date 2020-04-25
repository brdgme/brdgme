CREATE EXTENSION "uuid-ossp";

CREATE OR REPLACE FUNCTION update_updated_at()	
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now() AT TIME ZONE 'utc';
    RETURN NEW;	
END;
$$ language 'plpgsql';

CREATE TYPE color AS ENUM (
  'Green',
  'Red',
  'Blue',
  'Amber',
  'Purple',
  'Brown',
  'BlueGrey'
);

CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  created_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  updated_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  name TEXT NOT NULL UNIQUE,
  pref_colors TEXT[] NOT NULL,
  login_confirmation TEXT,
  login_confirmation_at TIMESTAMP
);
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users FOR EACH ROW EXECUTE PROCEDURE update_updated_at();

CREATE TABLE user_emails (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  created_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  updated_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  user_id UUID NOT NULL REFERENCES users (id),
  email TEXT NOT NULL UNIQUE,
  is_primary BOOL NOT NULL
);
CREATE TRIGGER update_user_emails_updated_at BEFORE UPDATE ON user_emails FOR EACH ROW EXECUTE PROCEDURE update_updated_at();

CREATE TABLE user_auth_tokens (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  created_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  updated_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  user_id UUID NOT NULL REFERENCES users (id)
);
CREATE TRIGGER update_user_auth_tokens_updated_at BEFORE UPDATE ON user_auth_tokens FOR EACH ROW EXECUTE PROCEDURE update_updated_at();

CREATE TABLE game_types (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  created_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  updated_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  name TEXT NOT NULL
);
CREATE TRIGGER update_game_types_updated_at BEFORE UPDATE ON game_types FOR EACH ROW EXECUTE PROCEDURE update_updated_at();

CREATE TABLE game_versions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  created_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  updated_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  game_type_id UUID NOT NULL REFERENCES game_types (id),
  name TEXT NOT NULL,
  uri TEXT NOT NULL,
  is_public BOOL NOT NULL,
  is_deprecated BOOL NOT NULL,
  UNIQUE (game_type_id, name)
);
CREATE TRIGGER update_game_versions_updated_at BEFORE UPDATE ON game_versions FOR EACH ROW EXECUTE PROCEDURE update_updated_at();

CREATE TABLE games (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  created_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  updated_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  game_version_id UUID NOT NULL REFERENCES game_versions (id),
  is_finished BOOL NOT NULL,
  finished_at TIMESTAMP,
  game_state TEXT NOT NULL
);
CREATE TRIGGER update_games_updated_at BEFORE UPDATE ON games FOR EACH ROW EXECUTE PROCEDURE update_updated_at();

CREATE TABLE game_players (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  created_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  updated_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  game_id UUID NOT NULL REFERENCES games (id),
  user_id UUID NOT NULL REFERENCES users (id),
  position INT NOT NULL,
  color TEXT NOT NULL,
  has_accepted BOOL NOT NULL,
  is_turn BOOL NOT NULL,
  is_turn_at TIMESTAMP NOT NULL,
  last_turn_at TIMESTAMP NOT NULL,
  is_eliminated BOOL NOT NULL,
  is_winner BOOL NOT NULL,
  is_read BOOL NOT NULL,
  points REAL,
  undo_game_state TEXT,
  UNIQUE (game_id, user_id),
  UNIQUE (game_id, color),
  UNIQUE (game_id, position)
);
CREATE TRIGGER update_game_players_updated_at BEFORE UPDATE ON game_players FOR EACH ROW EXECUTE PROCEDURE update_updated_at();

CREATE TABLE game_logs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  created_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  updated_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  game_id UUID NOT NULL REFERENCES games (id),
  body TEXT NOT NULL,
  is_public BOOL NOT NULL,
  logged_at TIMESTAMP NOT NULL
);
CREATE TRIGGER update_game_logs_updated_at BEFORE UPDATE ON game_logs FOR EACH ROW EXECUTE PROCEDURE update_updated_at();

CREATE TABLE game_log_targets (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  created_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  updated_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  game_log_id UUID NOT NULL REFERENCES game_logs (id),
  game_player_id UUID NOT NULL REFERENCES game_players (id)
);
CREATE TRIGGER update_game_log_targets_updated_at BEFORE UPDATE ON game_log_targets FOR EACH ROW EXECUTE PROCEDURE update_updated_at();

CREATE TABLE game_type_users (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  created_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  updated_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  game_type_id UUID NOT NULL REFERENCES game_types (id),
  user_id UUID NOT NULL REFERENCES users (id),
  last_game_finished_at TIMESTAMP,
  rating INT NOT NULL DEFAULT 1200,
  peak_rating INT NOT NULL DEFAULT 1200
);
CREATE TRIGGER update_game_type_users_updated_at BEFORE UPDATE ON game_type_users FOR EACH ROW EXECUTE PROCEDURE update_updated_at();

CREATE TABLE friends (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  created_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  updated_at TIMESTAMP NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
  source_user_id UUID NOT NULL REFERENCES users (id),
  target_user_id UUID NOT NULL REFERENCES users (id) CHECK (target_user_id != source_user_id),
  has_accepted BOOL
);
CREATE TRIGGER update_friends_updated_at BEFORE UPDATE ON friends FOR EACH ROW EXECUTE PROCEDURE update_updated_at();