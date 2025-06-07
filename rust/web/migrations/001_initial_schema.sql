--
-- PostgreSQL database dump
--

-- Dumped from database version 12.3 (Debian 12.3-1.pgdg100+1)
-- Dumped by pg_dump version 12.3 (Debian 12.3-1.pgdg100+1)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: uuid-ossp; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA public;


--
-- Name: EXTENSION "uuid-ossp"; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION "uuid-ossp" IS 'generate universally unique identifiers (UUIDs)';


--
-- Name: color; Type: TYPE; Schema: public; Owner: brdgme
--

CREATE TYPE public.color AS ENUM (
    'Green',
    'Red',
    'Blue',
    'Amber',
    'Purple',
    'Brown',
    'BlueGrey'
);


ALTER TYPE public.color OWNER TO brdgme;

--
-- Name: diesel_manage_updated_at(regclass); Type: FUNCTION; Schema: public; Owner: brdgme
--

CREATE OR REPLACE FUNCTION public.diesel_manage_updated_at(_tbl regclass) RETURNS void
    LANGUAGE plpgsql
    AS $$

BEGIN

    EXECUTE format('CREATE TRIGGER set_updated_at BEFORE UPDATE ON %s

                    FOR EACH ROW EXECUTE PROCEDURE diesel_set_updated_at()', _tbl);

END;
$$;


ALTER FUNCTION public.diesel_manage_updated_at(_tbl regclass) OWNER TO brdgme;

--
-- Name: diesel_set_updated_at(); Type: FUNCTION; Schema: public; Owner: brdgme
--

CREATE OR REPLACE FUNCTION public.diesel_set_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$

BEGIN

    IF (

        NEW IS DISTINCT FROM OLD AND

        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at

    ) THEN

        NEW.updated_at := current_timestamp;

    END IF;

    RETURN NEW;

END;
$$;


ALTER FUNCTION public.diesel_set_updated_at() OWNER TO brdgme;

--
-- Name: update_finished_at(); Type: FUNCTION; Schema: public; Owner: brdgme
--

CREATE OR REPLACE FUNCTION public.update_finished_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$

BEGIN

    NEW.finished_at = now() AT TIME ZONE 'utc';

    RETURN NEW;	

END;
$$;


ALTER FUNCTION public.update_finished_at() OWNER TO brdgme;

--
-- Name: update_is_turn_at(); Type: FUNCTION; Schema: public; Owner: brdgme
--

CREATE OR REPLACE FUNCTION public.update_is_turn_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$

BEGIN

    NEW.is_turn_at = now() AT TIME ZONE 'utc';

    RETURN NEW;	

END;
$$;


ALTER FUNCTION public.update_is_turn_at() OWNER TO brdgme;

--
-- Name: update_last_turn_at(); Type: FUNCTION; Schema: public; Owner: brdgme
--

CREATE OR REPLACE FUNCTION public.update_last_turn_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$

BEGIN

    NEW.last_turn_at = now() AT TIME ZONE 'utc';

    RETURN NEW;	

END;
$$;


ALTER FUNCTION public.update_last_turn_at() OWNER TO brdgme;

--
-- Name: update_updated_at(); Type: FUNCTION; Schema: public; Owner: brdgme
--

CREATE OR REPLACE FUNCTION public.update_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$

BEGIN

    NEW.updated_at = now() AT TIME ZONE 'utc';

    RETURN NEW;	

END;
$$;


ALTER FUNCTION public.update_updated_at() OWNER TO brdgme;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: __diesel_schema_migrations; Type: TABLE; Schema: public; Owner: brdgme
--

CREATE TABLE IF NOT EXISTS public.__diesel_schema_migrations (
    version character varying(50) NOT NULL,
    run_on timestamp without time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.__diesel_schema_migrations OWNER TO brdgme;

--
-- Name: chat_messages; Type: TABLE; Schema: public; Owner: brdgme
--

CREATE TABLE IF NOT EXISTS public.chat_messages (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    chat_user_id uuid NOT NULL,
    message text NOT NULL
);


ALTER TABLE public.chat_messages OWNER TO brdgme;

--
-- Name: chat_users; Type: TABLE; Schema: public; Owner: brdgme
--

CREATE TABLE IF NOT EXISTS public.chat_users (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    chat_id uuid NOT NULL,
    user_id uuid NOT NULL,
    last_read_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL
);


ALTER TABLE public.chat_users OWNER TO brdgme;

--
-- Name: chats; Type: TABLE; Schema: public; Owner: brdgme
--

CREATE TABLE IF NOT EXISTS public.chats (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL
);


ALTER TABLE public.chats OWNER TO brdgme;

--
-- Name: friends; Type: TABLE; Schema: public; Owner: brdgme
--

CREATE TABLE IF NOT EXISTS public.friends (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    source_user_id uuid NOT NULL,
    target_user_id uuid NOT NULL,
    has_accepted boolean,
    CONSTRAINT friends_check CHECK ((target_user_id <> source_user_id))
);


ALTER TABLE public.friends OWNER TO brdgme;

--
-- Name: game_log_targets; Type: TABLE; Schema: public; Owner: brdgme
--

CREATE TABLE IF NOT EXISTS public.game_log_targets (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    game_log_id uuid NOT NULL,
    game_player_id uuid NOT NULL
);


ALTER TABLE public.game_log_targets OWNER TO brdgme;

--
-- Name: game_logs; Type: TABLE; Schema: public; Owner: brdgme
--

CREATE TABLE IF NOT EXISTS public.game_logs (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    game_id uuid NOT NULL,
    body text NOT NULL,
    is_public boolean NOT NULL,
    logged_at timestamp without time zone NOT NULL
);


ALTER TABLE public.game_logs OWNER TO brdgme;

--
-- Name: game_players; Type: TABLE; Schema: public; Owner: brdgme
--

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


ALTER TABLE public.game_players OWNER TO brdgme;

--
-- Name: game_type_users; Type: TABLE; Schema: public; Owner: brdgme
--

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


ALTER TABLE public.game_type_users OWNER TO brdgme;

--
-- Name: game_types; Type: TABLE; Schema: public; Owner: brdgme
--

CREATE TABLE IF NOT EXISTS public.game_types (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    name text NOT NULL,
    player_counts integer[] DEFAULT ARRAY[]::integer[] NOT NULL,
    weight real DEFAULT 0 NOT NULL
);


ALTER TABLE public.game_types OWNER TO brdgme;

--
-- Name: game_versions; Type: TABLE; Schema: public; Owner: brdgme
--

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


ALTER TABLE public.game_versions OWNER TO brdgme;

--
-- Name: games; Type: TABLE; Schema: public; Owner: brdgme
--

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


ALTER TABLE public.games OWNER TO brdgme;

--
-- Name: user_auth_tokens; Type: TABLE; Schema: public; Owner: brdgme
--

CREATE TABLE IF NOT EXISTS public.user_auth_tokens (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    user_id uuid NOT NULL
);


ALTER TABLE public.user_auth_tokens OWNER TO brdgme;

--
-- Name: user_emails; Type: TABLE; Schema: public; Owner: brdgme
--

CREATE TABLE IF NOT EXISTS public.user_emails (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    user_id uuid NOT NULL,
    email text NOT NULL,
    is_primary boolean NOT NULL
);


ALTER TABLE public.user_emails OWNER TO brdgme;

--
-- Name: users; Type: TABLE; Schema: public; Owner: brdgme
--

CREATE TABLE IF NOT EXISTS public.users (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    name text NOT NULL,
    pref_colors text[] NOT NULL,
    login_confirmation text,
    login_confirmation_at timestamp without time zone
);


ALTER TABLE public.users OWNER TO brdgme;

--
-- Name: __diesel_schema_migrations __diesel_schema_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.__diesel_schema_migrations
    ADD CONSTRAINT __diesel_schema_migrations_pkey PRIMARY KEY (version);


--
-- Name: chat_messages chat_messages_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.chat_messages
    ADD CONSTRAINT chat_messages_pkey PRIMARY KEY (id);


--
-- Name: chat_users chat_users_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.chat_users
    ADD CONSTRAINT chat_users_pkey PRIMARY KEY (id);


--
-- Name: chats chats_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.chats
    ADD CONSTRAINT chats_pkey PRIMARY KEY (id);


--
-- Name: friends friends_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.friends
    ADD CONSTRAINT friends_pkey PRIMARY KEY (id);


--
-- Name: game_log_targets game_log_targets_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.game_log_targets
    ADD CONSTRAINT game_log_targets_pkey PRIMARY KEY (id);


--
-- Name: game_logs game_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.game_logs
    ADD CONSTRAINT game_logs_pkey PRIMARY KEY (id);


--
-- Name: game_players game_players_game_id_color_key; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.game_players
    ADD CONSTRAINT game_players_game_id_color_key UNIQUE (game_id, color);


--
-- Name: game_players game_players_game_id_position_key; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.game_players
    ADD CONSTRAINT game_players_game_id_position_key UNIQUE (game_id, "position");


--
-- Name: game_players game_players_game_id_user_id_key; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.game_players
    ADD CONSTRAINT game_players_game_id_user_id_key UNIQUE (game_id, user_id);


--
-- Name: game_players game_players_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.game_players
    ADD CONSTRAINT game_players_pkey PRIMARY KEY (id);


--
-- Name: game_type_users game_type_users_game_type_id_user_id_key; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.game_type_users
    ADD CONSTRAINT game_type_users_game_type_id_user_id_key UNIQUE (game_type_id, user_id);


--
-- Name: game_type_users game_type_users_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.game_type_users
    ADD CONSTRAINT game_type_users_pkey PRIMARY KEY (id);


--
-- Name: game_types game_types_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.game_types
    ADD CONSTRAINT game_types_pkey PRIMARY KEY (id);


--
-- Name: game_versions game_versions_game_type_id_name_key; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.game_versions
    ADD CONSTRAINT game_versions_game_type_id_name_key UNIQUE (game_type_id, name);


--
-- Name: game_versions game_versions_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.game_versions
    ADD CONSTRAINT game_versions_pkey PRIMARY KEY (id);


--
-- Name: games games_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.games
    ADD CONSTRAINT games_pkey PRIMARY KEY (id);


--
-- Name: user_auth_tokens user_auth_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.user_auth_tokens
    ADD CONSTRAINT user_auth_tokens_pkey PRIMARY KEY (id);


--
-- Name: user_emails user_emails_email_key; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.user_emails
    ADD CONSTRAINT user_emails_email_key UNIQUE (email);


--
-- Name: user_emails user_emails_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.user_emails
    ADD CONSTRAINT user_emails_pkey PRIMARY KEY (id);


--
-- Name: users users_name_key; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_name_key UNIQUE (name);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: brdgme
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: chat_messages update_chat_messages_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_chat_messages_updated_at BEFORE UPDATE ON public.chat_messages FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: chat_users update_chat_users_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_chat_users_updated_at BEFORE UPDATE ON public.chat_users FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: chats update_chats_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_chats_updated_at BEFORE UPDATE ON public.chats FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: games update_finished_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_finished_at BEFORE UPDATE ON public.games FOR EACH ROW WHEN (((old.is_finished = false) AND (new.is_finished = true))) EXECUTE FUNCTION public.update_finished_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: friends update_friends_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_friends_updated_at BEFORE UPDATE ON public.friends FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_log_targets update_game_log_targets_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_game_log_targets_updated_at BEFORE UPDATE ON public.game_log_targets FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_logs update_game_logs_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_game_logs_updated_at BEFORE UPDATE ON public.game_logs FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_players update_game_players_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_game_players_updated_at BEFORE UPDATE ON public.game_players FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_type_users update_game_type_users_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_game_type_users_updated_at BEFORE UPDATE ON public.game_type_users FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_types update_game_types_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_game_types_updated_at BEFORE UPDATE ON public.game_types FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_versions update_game_versions_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_game_versions_updated_at BEFORE UPDATE ON public.game_versions FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: games update_games_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_games_updated_at BEFORE UPDATE ON public.games FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_players update_is_turn_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_is_turn_at BEFORE UPDATE ON public.game_players FOR EACH ROW WHEN (((old.is_turn = false) AND (new.is_turn = true))) EXECUTE FUNCTION public.update_is_turn_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_players update_last_turn_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_last_turn_at BEFORE UPDATE ON public.game_players FOR EACH ROW WHEN (((old.is_turn = true) AND (new.is_turn = false))) EXECUTE FUNCTION public.update_last_turn_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: user_auth_tokens update_user_auth_tokens_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_user_auth_tokens_updated_at BEFORE UPDATE ON public.user_auth_tokens FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: user_emails update_user_emails_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_user_emails_updated_at BEFORE UPDATE ON public.user_emails FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: users update_users_updated_at; Type: TRIGGER; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON public.users FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: chat_messages chat_messages_chat_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.chat_messages
        ADD CONSTRAINT chat_messages_chat_user_id_fkey FOREIGN KEY (chat_user_id) REFERENCES public.chat_users(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: chat_users chat_users_chat_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.chat_users
        ADD CONSTRAINT chat_users_chat_id_fkey FOREIGN KEY (chat_id) REFERENCES public.chats(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: chat_users chat_users_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.chat_users
        ADD CONSTRAINT chat_users_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: friends friends_source_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.friends
        ADD CONSTRAINT friends_source_user_id_fkey FOREIGN KEY (source_user_id) REFERENCES public.users(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: friends friends_target_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.friends
        ADD CONSTRAINT friends_target_user_id_fkey FOREIGN KEY (target_user_id) REFERENCES public.users(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_log_targets game_log_targets_game_log_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.game_log_targets
        ADD CONSTRAINT game_log_targets_game_log_id_fkey FOREIGN KEY (game_log_id) REFERENCES public.game_logs(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_log_targets game_log_targets_game_player_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.game_log_targets
        ADD CONSTRAINT game_log_targets_game_player_id_fkey FOREIGN KEY (game_player_id) REFERENCES public.game_players(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_logs game_logs_game_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.game_logs
        ADD CONSTRAINT game_logs_game_id_fkey FOREIGN KEY (game_id) REFERENCES public.games(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_players game_players_game_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.game_players
        ADD CONSTRAINT game_players_game_id_fkey FOREIGN KEY (game_id) REFERENCES public.games(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_players game_players_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.game_players
        ADD CONSTRAINT game_players_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_type_users game_type_users_game_type_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.game_type_users
        ADD CONSTRAINT game_type_users_game_type_id_fkey FOREIGN KEY (game_type_id) REFERENCES public.game_types(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_type_users game_type_users_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.game_type_users
        ADD CONSTRAINT game_type_users_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: game_versions game_versions_game_type_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.game_versions
        ADD CONSTRAINT game_versions_game_type_id_fkey FOREIGN KEY (game_type_id) REFERENCES public.game_types(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: games games_chat_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.games
        ADD CONSTRAINT games_chat_id_fkey FOREIGN KEY (chat_id) REFERENCES public.chats(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: games games_game_version_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.games
        ADD CONSTRAINT games_game_version_id_fkey FOREIGN KEY (game_version_id) REFERENCES public.game_versions(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: games games_restarted_game_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.games
        ADD CONSTRAINT games_restarted_game_id_fkey FOREIGN KEY (restarted_game_id) REFERENCES public.games(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: user_auth_tokens user_auth_tokens_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.user_auth_tokens
        ADD CONSTRAINT user_auth_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- Name: user_emails user_emails_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: brdgme
--

DO $$
BEGIN
    ALTER TABLE ONLY public.user_emails
        ADD CONSTRAINT user_emails_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END$$;


--
-- PostgreSQL database dump complete
--

