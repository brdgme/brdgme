# Current Status

## Milestone: Phase 9 bot prompt complete and validated end-to-end

The bot can now make legal moves in a live Acquire game. Validated manually
against game `b1c38ce4-e6e7-4304-8dda-a9ee47d3915a` — bot played `F11` (first
attempt, no retries needed).

### What was done this session

- **`system_prompt.md`** fully written: all 10 `Spec` variants documented with
  YAML + plain-text examples, real Acquire worked example, difficulty levels,
  player loop with scores and colours, HTML game render, HTML logs, YAML command
  spec, conditional failed-commands block.
- **`rust/bot/src/prompt.rs`** created: `markup_to_html` (brdgme markup →
  HTML, resolves `{{player N}}` refs), `spec_to_yaml` (JSON→YAML roundtrip to
  avoid native YAML tags), `render_prompt` (MiniJinja), 15 unit tests.
- **`rust/bot/src/main.rs`** wired up: `BotContext` extended with
  `render_html`, `command_spec_yaml`, `recent_logs_html`, `points`. Retry loop
  accumulates `FailedCommand` entries and re-renders template each attempt.
- **`devenv.nix`**: added `retry` (nixpkgs retry binary) and `brdgme_color`
  dep in bot crate.
- **`scripts/setup-kind-cluster.sh`**: fixed idempotency for stopped cluster
  (start `kind-control-plane` container and wait for API server); replaced
  custom `retry()` shell function with `retry` binary from nixpkgs on the three
  `kubectl patch` calls that hit the Knative admission webhook.
- **`acquire-1/RULES.md`**: corrected 2-player dummy-shareholder rule — share
  count uses a D6 roll (1-6), not drawn tile column (1-12).
- **`docs/`**: removed scratch files `acquire1_status.json` and
  `command_spec.yaml`.

### Next steps (in order)

1. **Rules for other games** — write `RULES.md` for `lords-of-vegas-1`,
   `lost-cities-1`, `lost-cities-2` once bot is confirmed stable on Acquire
   across more games.

2. **KV cache restructure** (PLAN.md Phase 9) — split prompt into multiple
   messages with static content first to enable Ollama prefix caching. Low
   priority until bot quality is validated.

3. **Optimistic locking** (PLAN.md Phase 9) — `execute_command` +
   `update_game_command_success` race condition fix.

4. **Phase 6.5** — Production CD (ArgoCD + separate `brdgme-config` repo).

5. **Phase 7** — Side-by-side validation (old + new live together, then
   decommission legacy services).
