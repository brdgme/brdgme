-- Seed data for the E2E smoke suite: one game type + version pointing at the
-- locally-running lost_cities_2_http game service.
INSERT INTO game_types (id, name, player_counts)
VALUES ('00000000-0000-0000-0000-000000000001', 'Lost Cities', '{2,3}')
ON CONFLICT (id) DO NOTHING;

INSERT INTO game_versions (id, game_type_id, name, uri, is_public, is_deprecated)
VALUES (
    '00000000-0000-0000-0000-000000000002',
    '00000000-0000-0000-0000-000000000001',
    'Lost Cities',
    'http://127.0.0.1:8100',
    true,
    false
)
ON CONFLICT (id) DO NOTHING;
