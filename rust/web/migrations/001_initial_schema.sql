-- Initial schema migration - exact replication of production schema as of 2026-03-21.
-- All statements are idempotent and safe to run on an existing production database.
-- Diesel ORM artifacts (diesel_set_updated_at, diesel_manage_updated_at, set_updated_at
-- triggers) are removed in favour of the standard update_updated_at() trigger pattern.

-- Extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Types
DO $$ BEGIN
    CREATE TYPE public.color AS ENUM (
        'Green',
        'Red',
        'Blue',
        'Amber',
        'Purple',
        'Brown',
        'BlueGrey'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Functions
CREATE OR REPLACE FUNCTION public.update_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = now() AT TIME ZONE 'utc';
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.update_finished_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.finished_at = now() AT TIME ZONE 'utc';
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.update_is_turn_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.is_turn_at = now() AT TIME ZONE 'utc';
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.update_last_turn_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.last_turn_at = now() AT TIME ZONE 'utc';
    RETURN NEW;
END;
$$;

-- Remove Diesel ORM artifacts
DROP TRIGGER IF EXISTS set_updated_at ON public.users;
DROP TRIGGER IF EXISTS set_updated_at ON public.user_emails;
DROP TRIGGER IF EXISTS set_updated_at ON public.user_auth_tokens;
DROP TRIGGER IF EXISTS set_updated_at ON public.friends;
DROP TRIGGER IF EXISTS set_updated_at ON public.chats;
DROP TRIGGER IF EXISTS set_updated_at ON public.chat_users;
DROP TRIGGER IF EXISTS set_updated_at ON public.chat_messages;
DROP TRIGGER IF EXISTS set_updated_at ON public.game_types;
DROP TRIGGER IF EXISTS set_updated_at ON public.game_type_users;
DROP TRIGGER IF EXISTS set_updated_at ON public.game_versions;
DROP TRIGGER IF EXISTS set_updated_at ON public.games;
DROP TRIGGER IF EXISTS set_updated_at ON public.game_players;
DROP TRIGGER IF EXISTS set_updated_at ON public.game_logs;
DROP TRIGGER IF EXISTS set_updated_at ON public.game_log_targets;
DROP FUNCTION IF EXISTS public.diesel_manage_updated_at(regclass);
DROP FUNCTION IF EXISTS public.diesel_set_updated_at();

-- Tables

CREATE TABLE IF NOT EXISTS public.users (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    name text NOT NULL,
    pref_colors text[] NOT NULL,
    login_confirmation text,
    login_confirmation_at timestamp without time zone
);

CREATE TABLE IF NOT EXISTS public.user_emails (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    user_id uuid NOT NULL,
    email text NOT NULL,
    is_primary boolean NOT NULL
);

CREATE TABLE IF NOT EXISTS public.user_auth_tokens (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    user_id uuid NOT NULL
);

CREATE TABLE IF NOT EXISTS public.friends (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    source_user_id uuid NOT NULL,
    target_user_id uuid NOT NULL,
    has_accepted boolean,
    CONSTRAINT friends_check CHECK ((target_user_id <> source_user_id))
);

CREATE TABLE IF NOT EXISTS public.chats (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL
);

CREATE TABLE IF NOT EXISTS public.chat_users (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    chat_id uuid NOT NULL,
    user_id uuid NOT NULL,
    last_read_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL
);

CREATE TABLE IF NOT EXISTS public.chat_messages (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    chat_user_id uuid NOT NULL,
    message text NOT NULL
);

CREATE TABLE IF NOT EXISTS public.game_types (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    name text NOT NULL,
    player_counts integer[] DEFAULT ARRAY[]::integer[] NOT NULL,
    weight real DEFAULT 0 NOT NULL
);

CREATE TABLE IF NOT EXISTS public.game_type_users (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    game_type_id uuid NOT NULL,
    user_id uuid NOT NULL,
    last_game_finished_at timestamp without time zone,
    rating integer DEFAULT 1200 NOT NULL,
    peak_rating integer DEFAULT 1200 NOT NULL
);

CREATE TABLE IF NOT EXISTS public.game_versions (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    game_type_id uuid NOT NULL,
    name text NOT NULL,
    uri text NOT NULL,
    is_public boolean NOT NULL,
    is_deprecated boolean NOT NULL
);

CREATE TABLE IF NOT EXISTS public.games (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    game_version_id uuid NOT NULL,
    is_finished boolean NOT NULL,
    finished_at timestamp without time zone,
    game_state text NOT NULL,
    chat_id uuid,
    restarted_game_id uuid
);

CREATE TABLE IF NOT EXISTS public.game_players (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    game_id uuid NOT NULL,
    user_id uuid NOT NULL,
    "position" integer NOT NULL,
    color text NOT NULL,
    has_accepted boolean NOT NULL,
    is_turn boolean NOT NULL,
    is_turn_at timestamp without time zone NOT NULL,
    last_turn_at timestamp without time zone NOT NULL,
    is_eliminated boolean NOT NULL,
    is_read boolean NOT NULL,
    points real,
    undo_game_state text,
    place integer,
    rating_change integer
);

CREATE TABLE IF NOT EXISTS public.game_logs (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    game_id uuid NOT NULL,
    body text NOT NULL,
    is_public boolean NOT NULL,
    logged_at timestamp without time zone NOT NULL
);

CREATE TABLE IF NOT EXISTS public.game_log_targets (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    game_log_id uuid NOT NULL,
    game_player_id uuid NOT NULL
);

-- Primary key constraints
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'users_pkey') THEN
        ALTER TABLE ONLY public.users ADD CONSTRAINT users_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'user_emails_pkey') THEN
        ALTER TABLE ONLY public.user_emails ADD CONSTRAINT user_emails_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'user_auth_tokens_pkey') THEN
        ALTER TABLE ONLY public.user_auth_tokens ADD CONSTRAINT user_auth_tokens_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'friends_pkey') THEN
        ALTER TABLE ONLY public.friends ADD CONSTRAINT friends_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chats_pkey') THEN
        ALTER TABLE ONLY public.chats ADD CONSTRAINT chats_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chat_users_pkey') THEN
        ALTER TABLE ONLY public.chat_users ADD CONSTRAINT chat_users_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chat_messages_pkey') THEN
        ALTER TABLE ONLY public.chat_messages ADD CONSTRAINT chat_messages_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_types_pkey') THEN
        ALTER TABLE ONLY public.game_types ADD CONSTRAINT game_types_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_type_users_pkey') THEN
        ALTER TABLE ONLY public.game_type_users ADD CONSTRAINT game_type_users_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_versions_pkey') THEN
        ALTER TABLE ONLY public.game_versions ADD CONSTRAINT game_versions_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'games_pkey') THEN
        ALTER TABLE ONLY public.games ADD CONSTRAINT games_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_players_pkey') THEN
        ALTER TABLE ONLY public.game_players ADD CONSTRAINT game_players_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_logs_pkey') THEN
        ALTER TABLE ONLY public.game_logs ADD CONSTRAINT game_logs_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_log_targets_pkey') THEN
        ALTER TABLE ONLY public.game_log_targets ADD CONSTRAINT game_log_targets_pkey PRIMARY KEY (id);
    END IF;
END$$;

-- Unique constraints
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'users_name_key') THEN
        ALTER TABLE ONLY public.users ADD CONSTRAINT users_name_key UNIQUE (name);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'user_emails_email_key') THEN
        ALTER TABLE ONLY public.user_emails ADD CONSTRAINT user_emails_email_key UNIQUE (email);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_type_users_game_type_id_user_id_key') THEN
        ALTER TABLE ONLY public.game_type_users ADD CONSTRAINT game_type_users_game_type_id_user_id_key UNIQUE (game_type_id, user_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_versions_game_type_id_name_key') THEN
        ALTER TABLE ONLY public.game_versions ADD CONSTRAINT game_versions_game_type_id_name_key UNIQUE (game_type_id, name);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_players_game_id_color_key') THEN
        ALTER TABLE ONLY public.game_players ADD CONSTRAINT game_players_game_id_color_key UNIQUE (game_id, color);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_players_game_id_position_key') THEN
        ALTER TABLE ONLY public.game_players ADD CONSTRAINT game_players_game_id_position_key UNIQUE (game_id, "position");
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_players_game_id_user_id_key') THEN
        ALTER TABLE ONLY public.game_players ADD CONSTRAINT game_players_game_id_user_id_key UNIQUE (game_id, user_id);
    END IF;
END$$;

-- Foreign key constraints
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'user_emails_user_id_fkey') THEN
        ALTER TABLE ONLY public.user_emails
            ADD CONSTRAINT user_emails_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'user_auth_tokens_user_id_fkey') THEN
        ALTER TABLE ONLY public.user_auth_tokens
            ADD CONSTRAINT user_auth_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'friends_source_user_id_fkey') THEN
        ALTER TABLE ONLY public.friends
            ADD CONSTRAINT friends_source_user_id_fkey FOREIGN KEY (source_user_id) REFERENCES public.users(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'friends_target_user_id_fkey') THEN
        ALTER TABLE ONLY public.friends
            ADD CONSTRAINT friends_target_user_id_fkey FOREIGN KEY (target_user_id) REFERENCES public.users(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chat_users_chat_id_fkey') THEN
        ALTER TABLE ONLY public.chat_users
            ADD CONSTRAINT chat_users_chat_id_fkey FOREIGN KEY (chat_id) REFERENCES public.chats(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chat_users_user_id_fkey') THEN
        ALTER TABLE ONLY public.chat_users
            ADD CONSTRAINT chat_users_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'chat_messages_chat_user_id_fkey') THEN
        ALTER TABLE ONLY public.chat_messages
            ADD CONSTRAINT chat_messages_chat_user_id_fkey FOREIGN KEY (chat_user_id) REFERENCES public.chat_users(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_type_users_game_type_id_fkey') THEN
        ALTER TABLE ONLY public.game_type_users
            ADD CONSTRAINT game_type_users_game_type_id_fkey FOREIGN KEY (game_type_id) REFERENCES public.game_types(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_type_users_user_id_fkey') THEN
        ALTER TABLE ONLY public.game_type_users
            ADD CONSTRAINT game_type_users_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_versions_game_type_id_fkey') THEN
        ALTER TABLE ONLY public.game_versions
            ADD CONSTRAINT game_versions_game_type_id_fkey FOREIGN KEY (game_type_id) REFERENCES public.game_types(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'games_game_version_id_fkey') THEN
        ALTER TABLE ONLY public.games
            ADD CONSTRAINT games_game_version_id_fkey FOREIGN KEY (game_version_id) REFERENCES public.game_versions(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'games_chat_id_fkey') THEN
        ALTER TABLE ONLY public.games
            ADD CONSTRAINT games_chat_id_fkey FOREIGN KEY (chat_id) REFERENCES public.chats(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'games_restarted_game_id_fkey') THEN
        ALTER TABLE ONLY public.games
            ADD CONSTRAINT games_restarted_game_id_fkey FOREIGN KEY (restarted_game_id) REFERENCES public.games(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_players_game_id_fkey') THEN
        ALTER TABLE ONLY public.game_players
            ADD CONSTRAINT game_players_game_id_fkey FOREIGN KEY (game_id) REFERENCES public.games(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_players_user_id_fkey') THEN
        ALTER TABLE ONLY public.game_players
            ADD CONSTRAINT game_players_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_logs_game_id_fkey') THEN
        ALTER TABLE ONLY public.game_logs
            ADD CONSTRAINT game_logs_game_id_fkey FOREIGN KEY (game_id) REFERENCES public.games(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_log_targets_game_log_id_fkey') THEN
        ALTER TABLE ONLY public.game_log_targets
            ADD CONSTRAINT game_log_targets_game_log_id_fkey FOREIGN KEY (game_log_id) REFERENCES public.game_logs(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_log_targets_game_player_id_fkey') THEN
        ALTER TABLE ONLY public.game_log_targets
            ADD CONSTRAINT game_log_targets_game_player_id_fkey FOREIGN KEY (game_player_id) REFERENCES public.game_players(id);
    END IF;
END$$;

-- Indexes
CREATE INDEX IF NOT EXISTS idx_user_emails_user_id ON public.user_emails(user_id);
CREATE INDEX IF NOT EXISTS idx_user_auth_tokens_user_id ON public.user_auth_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_friends_source_user_id ON public.friends(source_user_id);
CREATE INDEX IF NOT EXISTS idx_friends_target_user_id ON public.friends(target_user_id);
CREATE INDEX IF NOT EXISTS idx_chat_users_chat_id ON public.chat_users(chat_id);
CREATE INDEX IF NOT EXISTS idx_chat_users_user_id ON public.chat_users(user_id);
CREATE INDEX IF NOT EXISTS idx_chat_messages_chat_user_id ON public.chat_messages(chat_user_id);
CREATE INDEX IF NOT EXISTS idx_game_type_users_game_type_id ON public.game_type_users(game_type_id);
CREATE INDEX IF NOT EXISTS idx_game_type_users_user_id ON public.game_type_users(user_id);
CREATE INDEX IF NOT EXISTS idx_game_versions_game_type_id ON public.game_versions(game_type_id);
CREATE INDEX IF NOT EXISTS idx_games_game_version_id ON public.games(game_version_id);
CREATE INDEX IF NOT EXISTS idx_games_is_finished ON public.games(is_finished);
CREATE INDEX IF NOT EXISTS idx_game_players_game_id ON public.game_players(game_id);
CREATE INDEX IF NOT EXISTS idx_game_players_user_id ON public.game_players(user_id);
CREATE INDEX IF NOT EXISTS idx_game_players_is_turn ON public.game_players(is_turn);
CREATE INDEX IF NOT EXISTS idx_game_logs_game_id ON public.game_logs(game_id);
CREATE INDEX IF NOT EXISTS idx_game_log_targets_game_log_id ON public.game_log_targets(game_log_id);
CREATE INDEX IF NOT EXISTS idx_game_log_targets_game_player_id ON public.game_log_targets(game_player_id);

-- Triggers
CREATE OR REPLACE TRIGGER update_users_updated_at
    BEFORE UPDATE ON public.users
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_user_emails_updated_at
    BEFORE UPDATE ON public.user_emails
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_user_auth_tokens_updated_at
    BEFORE UPDATE ON public.user_auth_tokens
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_friends_updated_at
    BEFORE UPDATE ON public.friends
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_chats_updated_at
    BEFORE UPDATE ON public.chats
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_chat_users_updated_at
    BEFORE UPDATE ON public.chat_users
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_chat_messages_updated_at
    BEFORE UPDATE ON public.chat_messages
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_game_types_updated_at
    BEFORE UPDATE ON public.game_types
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_game_type_users_updated_at
    BEFORE UPDATE ON public.game_type_users
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_game_versions_updated_at
    BEFORE UPDATE ON public.game_versions
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_games_updated_at
    BEFORE UPDATE ON public.games
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_game_players_updated_at
    BEFORE UPDATE ON public.game_players
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_game_logs_updated_at
    BEFORE UPDATE ON public.game_logs
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_game_log_targets_updated_at
    BEFORE UPDATE ON public.game_log_targets
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

CREATE OR REPLACE TRIGGER update_finished_at
    BEFORE UPDATE ON public.games
    FOR EACH ROW
    WHEN ((old.is_finished = false) AND (new.is_finished = true))
    EXECUTE FUNCTION public.update_finished_at();

CREATE OR REPLACE TRIGGER update_is_turn_at
    BEFORE UPDATE ON public.game_players
    FOR EACH ROW
    WHEN ((old.is_turn = false) AND (new.is_turn = true))
    EXECUTE FUNCTION public.update_is_turn_at();

CREATE OR REPLACE TRIGGER update_last_turn_at
    BEFORE UPDATE ON public.game_players
    FOR EACH ROW
    WHEN ((old.is_turn = true) AND (new.is_turn = false))
    EXECUTE FUNCTION public.update_last_turn_at();
