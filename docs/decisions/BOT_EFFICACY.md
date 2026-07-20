# Bot Efficacy (#43) - Key Decisions

**Implemented 2026-07-20.** Structured YAML data replaces the full game
render in bot prompts. All 27 Rust games upgraded to V2 interface.

## Decisions

**YAML format for bot data (not JSON, not render).** Player and public
state are re-serialized from JSON to YAML before inclusion in the bot
prompt. YAML is more token-efficient (no braces/quotes noise) and more
readable for LLMs. The render (brdgme markup) is never sent to bots.

**Data docs auto-generated from code (planned), hand-written (interim).**
The spec calls for a proc-macro derive that generates DATA_DOCS.md from
doc comments on PubState/PlayerState fields. Not yet built - all 27
games have hand-written DATA_DOCS.md files. Doc comments are already on
the struct fields as the source of truth.

**EXAMPLES.md dropped.** The render-explanation doc (working name
EXAMPLES.md from the original #43 scope) was dropped entirely. Bots
never see the render, so explaining its format is irrelevant.

**BASIC_STRATEGY + ADVANCED_STRATEGY split.** Two separate docs rather
than one STRATEGY doc. BASIC = short list of absolute don'ts (moves
that are almost always wrong). ADVANCED = longer contextual heuristics
for strong play. Bots receive BASIC at all difficulty levels; ADVANCED
only when the bot config's `include_advanced_strategy` flag is set.

**Bot config in DB (not env vars, not config file).** The bots,
llm_providers, and bot_providers tables hold all bot configuration.
This enables runtime changes without redeployment and lays groundwork
for the admin GUI. Env var fallback exists for dev convenience (when
bots table is empty).

**bot_name not constrained to easy/medium/hard.** The old
`game_bots.difficulty` column (CHECK constraint limiting values) is
renamed to `bot_name` with no constraint. Arbitrary bot names are
allowed - the seeded easy/medium/hard are just the initial set.

**Provider credentials: AES-256-GCM field encryption.**
llm_providers.api_key_encrypted stores the AES-256-GCM ciphertext
(nonce prepended). Key from DATABASE_ENCRYPTION_KEY env var (hex-encoded 32
bytes). Chosen over sealed-secrets (per-field granularity needed) and
Vault (operational overhead unjustified at this scale).

**Game interface versioning in game_client (callers never see
versions).** The game_client crate's `fetch_game_data()` calls Status
plus V2 endpoints and returns empty placeholder strings for V1 games.
Web and bot code calls `fetch_game_data()` unconditionally - no version
checks, no branching.

**splendor-2 keeps inline cost.rs (not migrated to brdgme_cost).**
splendor-2's existing cost.rs predates the shared brdgme_cost crate and
works correctly. Migrating it is churn with no benefit. Two coexisting
implementations are accepted.

## Tech Debt

- **DATA_DOCS.md hand-written per game.** Spec calls for proc-macro
  derive auto-generation from struct doc comments. Not yet built.
- **Bot `#[allow(dead_code)]` on config/crypto modules.** The API
  surface (load_providers, decrypt, ProviderRouter) exceeds what the
  current single-binary bot uses. Will be exercised by the admin GUI.
- **Bot env var fallback path.** When the bots table is empty, config
  falls back to BOT_MODEL/BOT_API_KEY/BOT_BASE_URL env vars. Dev
  convenience - remove once DB seeding is standard in all environments.
- **.sqlx cache manually edited for migration 013.** Cache entries were
  hand-written (not regenerated against a live DB). Needs verification
  with `cargo sqlx prepare` against a running database.
- **lords-of-vegas-1 has no k8s manifests.** Implemented in rust/game/
  but intentionally not deployed (no Tiltfile entry, no k8s manifests).
  Pre-existing, not caused by #43.

## Discoveries

- **Go starship_catan Winners() bug.** Returned the same player index
  for both branches of the win-condition check. Fixed in the Rust port.
- **Go starship_catan die-range off-by-one.** `%3+1` produces {1,2,3},
  not {1,2,3,4} as the rules require. Fixed in the Rust port.
- **Go Cost.Drop bug.** Iterated the index variable instead of the
  values when dropping resources. Fixed in brdgme_cost.
- **Rating bug: apply_rating_changes skipped ENTIRE game if any bot
  present.** Humans playing in bot games never received rating changes.
  Fixed: only bot players are excluded from the rated set.
- **Reciprocal friend request auto-accept was already implemented.** No
  change needed - the existing code handles it correctly.
- **Bot was receiving full render; structured data already available.**
  The Status response already includes player_state/pub_state fields,
  but the bot prompt was using the full render string. This motivated
  the #43 restructure.
