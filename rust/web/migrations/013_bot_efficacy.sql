CREATE TABLE bots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL UNIQUE,
    display_order INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT true,
    include_basic_strategy BOOLEAN NOT NULL DEFAULT true,
    include_advanced_strategy BOOLEAN NOT NULL DEFAULT false,
    temperature REAL NOT NULL DEFAULT 0.2,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE llm_providers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL UNIQUE,
    url TEXT NOT NULL,
    api_key_encrypted BYTEA,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE bot_providers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    provider_id UUID NOT NULL REFERENCES llm_providers(id) ON DELETE CASCADE,
    model TEXT NOT NULL,
    reasoning_effort TEXT,
    extra_body JSONB,
    priority INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (bot_id, provider_id, model)
);

ALTER TABLE game_bots RENAME COLUMN difficulty TO bot_name;
ALTER TABLE game_bots DROP CONSTRAINT IF EXISTS game_bots_difficulty_check;

ALTER TABLE game_versions ADD COLUMN interface_version INTEGER NOT NULL DEFAULT 1;

INSERT INTO bots (name, display_order, include_basic_strategy, include_advanced_strategy, temperature)
VALUES
    ('easy',   0, true,  false, 0.2),
    ('medium', 1, true,  false, 0.2),
    ('hard',   2, true,  true,  0.2);
