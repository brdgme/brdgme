# W6 triage: bot-operator-tools + dependencies

## Unit: bot-operator-tools
Extracted tally: 31 findings (0c / 4 major / 16 minor / 11 nit).

DISCREPANCY: expected 30 (0c/4M/15m/11n) per unit tally; actual finding headings counted = 31
(0c/4M/16m/11n). The unit's own per-section tally claims bot = 16 (2M/9m/5n) but the bot
sections contain 17 headings (2M/10m/5n). Operator (8: 1M/4m/3n) and tools (6: 1M/2m/3n)
match. Rows below reflect the 31 actual headings.

## Unit: dependencies
Extracted tally: actual heading count = 27 (0c/4M/18m/5n).

DISCREPANCY: expected 26 (0c/4M/17m/5n); actual headings = 27 (0c/4M/18m/5n). The file's
tally note says "one chrono duplicate merged in curation" - the merged doc still carries 18
minor headings, so the stated 26 appears stale by one minor. Rows below reflect 27.

## Rows

bot-operator-tools F1 | major | Reachable unreachable!(): final retry attempt on a continue path panics spawned task | bot/src/main.rs | M | bot-retry-loop
bot-operator-tools F2 | major | No ack-deadline Progress heartbeat; long turns risk redelivery + duplicate commands | bot/src/main.rs | D | bot-ack-heartbeat (ack-heartbeat vs raising ack_wait; consumer config owned by monolith)
bot-operator-tools F3 | minor | Unbounded tokio::spawn per message, no in-process concurrency limit | bot/src/main.rs | M | bot-consumer-hardening
bot-operator-tools F4 | minor | merge_json_patch deviates from RFC 7396: nulls preserved when target entry absent/scalar | bot/src/main.rs | M | bot-merge-patch
bot-operator-tools F5 | minor | No graceful shutdown; SIGTERM aborts in-flight turns mid-LLM-call | bot/src/main.rs | M | bot-consumer-hardening
bot-operator-tools F6 | minor | "(you)" marker keys on name equality; duplicate player names break seat identity | bot/user_prompt.md, bot/src/main.rs | M | bot-prompt
bot-operator-tools F7 | nit | "Rendered prompt" trace logs only system message, not per-turn user message | bot/src/main.rs | M | bot-prompt
bot-operator-tools F8 | nit | /healthz checks NATS only, not DB pool | bot/src/main.rs | M | bot-consumer-hardening
bot-operator-tools F9 | minor | Pervasive try_get(...).unwrap_or(default) masks decode/schema errors | bot/src/config.rs, bot/src/main.rs | M | bot-config-errors
bot-operator-tools F10 | minor | LLM_EXTRA_BODY JSON parse errors silently yield None | bot/src/config.rs | M | bot-config-errors
bot-operator-tools F11 | nit | load_key silently returns insecure default key; warning derived separately | bot/src/crypto.rs | M | bot-crypto
bot-operator-tools F12 | nit | Bespoke nonce gen via direct getrandom instead of aes-gcm generate_nonce | bot/src/crypto.rs | M | bot-crypto
bot-operator-tools F13 | minor | Hand-rolled {{player N}} str::replace instead of brdgme_markup; sequential-substitution injection quirk | bot/src/prompt.rs | D | bot-prompt (use brdgme_markup transform vs document+drop dep)
bot-operator-tools F14 | minor | ProviderRouter::next is a peek; misleading Iterator-like API | bot/src/routing.rs | M | bot-routing
bot-operator-tools F15 | nit | minijinja Environment rebuilt per render; use static LazyLock | bot/src/prompt.rs | M | bot-prompt
bot-operator-tools F16 | minor | Unused deps/features: time, brdgme_markup, tokio "signal" feature | bot/Cargo.toml | M | bot-deps
bot-operator-tools F17 | minor | serde_yaml 0.9 unmaintained/archived (bot consumer) | bot/Cargo.toml, bot/src/prompt.rs | D | serde-yaml (fork choice, fix with deps F14)
bot-operator-tools F18 | major | Hand-rolled finalizer merge patch can clobber concurrent finalizer edits | operator/src/controller.rs | M | operator-finalizer
bot-operator-tools F19 | minor | CRD printcolumn jsonPath .spec.playerCounts targets nonexistent field | operator/src/crd.rs | M | operator-crd
bot-operator-tools F20 | minor | GameVersionStatus.message never written; errors never surface in CRD status | operator/src/crd.rs, operator/src/controller.rs | M | operator-status
bot-operator-tools F21 | minor | Status patched via ad-hoc json! instead of typed GameVersionStatus | operator/src/controller.rs | M | operator-status
bot-operator-tools F22 | minor | weight bound as f64 against real (float4) column; bind f32 directly | operator/src/controller.rs | M | operator-status
bot-operator-tools F23 | nit | Redundant serde rename on interface_version (rename_all already camelCase) | operator/src/crd.rs | M | operator-crd
bot-operator-tools F24 | nit | interceptor_uri test depends on ambient INTERCEPTOR_URI env | operator/src/controller.rs | M | operator-crd
bot-operator-tools F25 | nit | k8s-openapi "latest" feature instead of pinning cluster API version | operator/Cargo.toml | D | operator-deps (pin choice depends on deployed cluster version)
bot-operator-tools F26 | major | Fuzzer hangs forever if all workers die: main keeps live Sender, recv never disconnects | tools/fuzz/src/lib.rs | M | fuzz-hang
bot-operator-tools F27 | minor | SystemTime::duration_since for interval timing panics on clock step; use Instant | tools/fuzz/src/lib.rs | M | fuzz-hang
bot-operator-tools F28 | minor | num_cpus dep duplicates std available_parallelism | tools/fuzz/Cargo.toml, tools/fuzz/src/lib.rs | M | fuzz-deps
bot-operator-tools F29 | nit | Factory wrapped in Arc<Mutex<>> only to avoid Sync bound | tools/fuzz/src/lib.rs | M | fuzz-cleanup
bot-operator-tools F30 | nit | Exit-signal tx.send(()).unwrap() panics when worker died early | tools/fuzz/src/lib.rs | M | fuzz-hang
bot-operator-tools F31 | nit | Whole PlayerRender cloned to extract command_spec | tools/fuzz/src/lib.rs | M | fuzz-cleanup
dependencies F1 | major | No [workspace.dependencies]; shared versions copy-pasted across 40 manifests with drift | Cargo.toml | D | workspace-deps (migration scope/order; umbrella for version bumps)
dependencies F2 | minor | No [workspace.package]; metadata repeated, authors already inconsistent | Cargo.toml, bot/Cargo.toml, web/Cargo.toml, operator/Cargo.toml | D | workspace-deps
dependencies F3 | minor | No [workspace.lints] table; lint policy not enforceable workspace-wide | Cargo.toml | D | workspace-deps
dependencies F4 | minor | [profile.wasm-release] duplicated in web/Cargo.toml is ignored by cargo | web/Cargo.toml, Cargo.toml | M | cargo-hygiene
dependencies F5 | nit | Root members list unsorted; empty android-dev/server-dev profiles unused | Cargo.toml | M | cargo-hygiene
dependencies F6 | major | sqlx split 0.8 (web) vs 0.9 (bot/operator); two full sqlx stacks compiled | web/Cargo.toml, bot/Cargo.toml, operator/Cargo.toml | D | sqlx-unify (tower-sessions-sqlx-store bump vs vendoring session store)
dependencies F7 | minor | getrandom split 0.3 (bot) vs 0.4 (web); lock carries 0.2/0.3/0.4 | bot/Cargo.toml, web/Cargo.toml | M | bot-crypto (resolved by dropping bot's direct getrandom, botops F12)
dependencies F8 | minor | Three parallel rand stacks (0.8/0.9/0.10) in lock, all transitive | Cargo.lock | M | lock-duplicates (partly collapses with sqlx-unify; monitor only)
dependencies F9 | minor | web pins tower-http/gloo-net/gloo-timers one step ahead of ecosystem, doubling each | web/Cargo.toml | M | web-dep-pins
dependencies F10 | minor | chrono in rand_bot vs time everywhere else; two datetime libs for one consumer | lib/rand_bot/Cargo.toml | M | rand-bot-time
dependencies F11 | minor | All 27 game crates use tokio features=["full"] for a 13-line http bin | game/tic-tac-toe-2/Cargo.toml | D | game-bins (feature trim vs letting shared bin crate own tokio; couples to F26)
dependencies F12 | major | sentry default features drag actix-web 4 + ureq 3 into every server build | bot/Cargo.toml, web/Cargo.toml, lib/cmd/Cargo.toml, lib/game_client/Cargo.toml | D | sentry-trim (feature list choice; verify with cargo tree)
dependencies F13 | major | term_size 0.3.2 unmaintained (RUSTSEC-2020-0163), direct dep of brdgme_cmd | lib/cmd/Cargo.toml, deny.toml | M | term-size (replace with terminal_size, drop deny ignore)
dependencies F14 | minor | serde_yaml archived; two direct consumers (bot, game_client) must migrate together | lib/game_client/Cargo.toml, bot/Cargo.toml | D | serde-yaml (fork choice: serde_yaml_ng/serde-yml/saphyr vs JSON)
dependencies F15 | minor | combine 4.6 dormant at heart of markup/game parsing | lib/game/Cargo.toml, lib/markup/Cargo.toml | D | combine (winnow migration vs accepted risk)
dependencies F16 | minor | warp 0.4 for game-service HTTP while platform is on axum | lib/cmd/Cargo.toml | D | warp-axum
dependencies F17 | minor | env_logger hard dep in brdgme_cmd vs tracing in deployables; lib inits logger | lib/cmd/Cargo.toml | M | cmd-logging
dependencies F18 | minor | paste 1.0.15 unmaintained (RUSTSEC-2024-0436) transitive via leptos; ignore needs comment | Cargo.lock, deny.toml | M | deny-toml
dependencies F19 | minor | svix pulls both http 0.2 and http 1.x; sole legacy-http holdout after sentry fix | Cargo.lock, web/Cargo.toml | M | lock-duplicates (monitor at next refresh)
dependencies F20 | nit | num_cpus where std available_parallelism suffices (dup of botops F28) | tools/fuzz/Cargo.toml | M | fuzz-deps
dependencies F21 | nit | lazy_static superseded by std LazyLock; two first-party consumers | lib/color/Cargo.toml, game/lords-of-vegas-1/Cargo.toml | M | lazylock
dependencies F22 | nit | convert_case at three versions, all transitive; no action | Cargo.lock | M | lock-duplicates (record only)
dependencies F23 | nit | Currency claims unverified offline; add cargo outdated/deny CI job | Cargo.lock | M | deps-ci
dependencies F24 | minor | deny.toml bans multiple-versions/sources/wildcards only warn; duplicates land silently | deny.toml | D | deny-toml (hardening: deny + skip-list enumeration, ordering vs dedup fixes)
dependencies F25 | minor | 4 of 7 advisory ignores stale - diesel/encoding absent from lock | deny.toml | M | deny-toml
dependencies F26 | minor | 27 game crates x 4 boilerplate bins = 108 near-identical files | game/tic-tac-toe-2/src/bin/tic_tac_toe_2_cli.rs | D | game-bins (macro in lib/cmd vs generic bin crate)
dependencies F27 | minor | lib/cost has one consumer while splendor-2 reimplements cost locally | lib/cost/Cargo.toml, game/splendor-2/src/cost.rs | D | lib-cost (fold into seven-wonders-1 vs port splendor-2 onto it)

## Grouping notes

- workspace-deps (deps F1-F3) is the umbrella package: doing it first makes every subsequent
  version-unification fix (sqlx-unify, sentry-trim, getrandom, serde/tokio bumps) a one-line
  root edit instead of 30+ manifest edits. F1/F2/F3 should land as one migration PR.
- sqlx-unify (deps F6) gates lock-duplicates cleanup: the rand 0.8 and RustCrypto duplicate
  clusters (F8) partly disappear with it. Blocked on a tower-sessions-sqlx-store decision.
- sentry-trim (deps F12) removes the actix + ureq subtrees and makes svix (F19) the only
  http 0.2 holdout - do sentry before re-auditing the lock.
- deny-toml package: F13 (term_size replacement drops an ignore), F18 (comment paste ignore),
  F24 (warn->deny hardening), F25 (delete stale ignores) belong together, but F24's deny
  switch should land last, after the dedup fixes, so the skip-list starts minimal.
- bot-crypto crosses units: botops F12 (use aes-gcm generate_nonce) is the fix that resolves
  deps F7 (getrandom 0.3 drift) for bot; do them as one change.
- serde-yaml crosses units: botops F17 and deps F14 are the same decision; both consumers
  (bot, lib/game_client) must migrate together or the archived crate stays in the tree.
- fuzz-deps crosses units: botops F28 and deps F20 are the identical num_cpus finding.
- game-bins (deps F11 + F26): the 108-boilerplate-bins decision determines where the game
  crates' tokio/fuzz/cmd deps live; decide the macro-vs-bin-crate question before trimming
  tokio features 27 times.
- bot-ack-heartbeat (botops F2) is the same NATS ack-after-all-work pattern flagged in the
  web-server/web-domain units; whatever heartbeat/ack_wait policy is chosen should be applied
  uniformly across bot and web consumers, and the consumer config is owned by the monolith.
- bot-consumer-hardening (F3/F5/F8) is a natural single package: shutdown handling,
  concurrency bound, and healthz all touch the same consumer loop in bot/src/main.rs; F5 also
  closes botops F16's dangling tokio "signal" feature.
- fuzz-hang (botops F26/F27/F30) are all robustness fixes in the same channel/loop in
  tools/fuzz/src/lib.rs; fix in one pass with fuzz-cleanup (F29/F31) as optional riders.
- operator work splits cleanly: operator-finalizer (F18, standalone behavioral fix),
  operator-status (F20/F21/F22, same status-patch code path), operator-crd (F19/F23/F24,
  cosmetic/test hygiene).
