-- R-51 / F-196: nullable per-version snapshot columns on game_versions.
-- The authoritative-version guard only writes forward, so deprecating or
-- deleting the newest version permanently strands game_types descriptor
-- values. These columns snapshot player_counts, weight, and blurb per
-- game_version so game_types descriptor values can be re-pointed from the
-- newest fully snapshotted authoritative version. Nullable and additive to
-- keep old web/operator binaries compatible throughout a rolling deployment;
-- existing rows start with incomplete snapshots.
ALTER TABLE public.game_versions
    ADD COLUMN IF NOT EXISTS snapshot_player_counts integer[],
    ADD COLUMN IF NOT EXISTS snapshot_weight real,
    ADD COLUMN IF NOT EXISTS snapshot_blurb text;
