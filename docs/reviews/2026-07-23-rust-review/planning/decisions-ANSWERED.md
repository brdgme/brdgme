# Decisions ANSWERED - brdgme Rust review

> **INCOMPLETE - this file covers D-01..D-34 ONLY.**
>
> Planning session 3 (2026-07-26) added **D-41 through D-53**, which live in
> **`decisions-session3.md`** and have NOT been folded in here. They carry the
> same authority as the rulings below. Among them: the pivot from WebSockets to
> **SSE** (D-44), the two-stream topology measured as HTTP/2 (D-48), the
> repeatable topic parameter (D-50), the reversal of the `_fuzz` binary deletion
> (D-43), and the maximum-performance fuzzer deferred to `docs/BACKLOG.md` #54
> (D-51). Several of these **supersede** positions recorded elsewhere in the
> planning corpus.
>
> **Read `decisions-session3.md` alongside this file.** Do not treat this one as
> the complete decision record until the two are merged.

**All 34 open decisions are CLOSED (2026-07-26).** This file replaces
`open-decisions-for-user.md`, which was the open-questions form of the same
table. Row order is unchanged; each row now states the **ruling** plus any
constraint or rationale attached to it. This is the implementer's reference -
where a ruling contradicts an older recommendation in `decisions-needed.md`,
`work-packages.md`, a spec, or a checklist, **this file wins**.

Five rulings **changed** a previously recorded position. Do not follow the
superseded text:

| id | change |
|---|---|
| `D-7` | **OVERRULED.** No redacted user-facing export at all. |
| `D-8` | **REFINED.** Restart resolves to the latest non-deprecated bot, not a no-op. |
| `D-15` | **REDESIGNED.** Game parser first, platform commands as fallback. |
| `D-16` | **OVERRULED.** Full page load for `/login`, not an effect-driven `render()`. |
| `D-37` | **CORRECTED.** `{{lbrace}}`, not a bare `{{`. |

---

## The rulings

| id | ruling | constraints, rationale, and notes |
|---|---|---|
| `D-37` | **ACCEPTED - use `{{lbrace}}`.** Corrects D-37's own answer, which named a bare `{{` as the literal-brace escape. | A bare `{{` matches the leading `{{` of every closing tag, so nested markup eats its own terminator and cannot be implemented soundly. `{{lbrace}}` stays inside the `{{...}}` family the decision asked for. `}` needs no escape. The stored-content risk flagged in D-37 still stands: WP-02 assesses it by reading code and migrations only, **never** by querying a database. |
| `D-8` | **REFINED - on restart, resolve a deprecated bot to the LATEST NON-DEPRECATED version of that bot.** | Supersedes the recommendation ("exempt the restart path and fall into D-5's dangling-name no-op plus admin warning"). The no-op fallback is **not** the answer for restart. D-8's core answer is unchanged: validate on write, tolerate on read. The restart path now actively re-resolves rather than rejecting or no-opping. |
| `D-14` | **ACCEPTED - keep the 6-digit code; link-vs-code stays a non-goal.** | Michael: "low value UI we need to maintain into the future." No new package. |
| `bo F25` | **ANSWERED - deployed cluster is Kubernetes server v1.36.0** (client v1.36.2, kustomize v5.8.1). Pin `k8s-openapi` to the `v1_36` feature. | The implementer **must confirm `k8s-openapi` actually ships a `v1_36` feature flag at fix time.** If it does not, use the highest available flag at or below v1.36 and record the choice in the WP-62 spec. |
| `D-11` | **ACCEPTED - option A: `reminder_emails_enabled` alone governs reminders.** `turn_emails_enabled` governs turn notifications only. | Michael's rationale: some users play mainly by web and do not want turn emails, but reminders are still useful if they have missed or forgotten a game. Requiring both flags makes the reminder flag dead exactly for those users. |
| `D-15` | **ACCEPTED, and the design is now SETTLED.** Do **not** hardcode a reserved-verb list. On game-scoped messages, **try the game command parser FIRST; platform commands are the FALLBACK when the game parser fails.** One carve-out: a **small hard-reserved set of escape-hatch verbs** (`help` and equivalents) always wins, even on the game path. | Supersedes the recommendation ("A-plus": keep the reserved list plus a game-scoped override). Michael proposed the parser-first design; the Orchestrator ruled on it. The escape-hatch set exists so a game with a greedy parser cannot swallow the only command that unsticks a user. Keep that set small and obvious. This fixes the live defect where acquire-1 and starship-catan-1 players cannot issue `end` by email. Unblocks WP-59 Task 14, whose COMMANDS.md text must be rewritten to describe parser-first dispatch, not a reservation. |
| `D-7` | **OVERRULED - do NOT build a redacted user-facing export.** The **only** export path is the full bundle, **admin-only**. | Supersedes the recommendation (option A: `--redact-private`, default ON for a user-facing path). Both the `--redact-private` flag and the user-facing export path are **out of scope**. Bug reporting is by **game ID**. Michael explicitly accepts the risk that game state may change after a report and render it useless. WP-48's scope shrinks accordingly. |
| `D-9` | **ACCEPTED - option B:** boundary normalization **plus** the one-off migration lowercasing stored rows **plus** the lower-index unique constraint. | Boundary-only leaves existing mixed-case rows permanently unmatchable. Surface the collision risk (two accounts differing only by case) once, deliberately, during the migration. |
| `D-10` | **ACCEPTED - option A: build the HTTPS one-click unsubscribe endpoint**, tokenised, no auth redirect. **Plus an addition:** the mail must carry **two visible links**. | The two visible links are (1) a **type-specific** "Unsubscribe from game reminders" matching the email type actually received, and (2) a "Manage my subscriptions" link to the user settings page. The `List-Unsubscribe` / `List-Unsubscribe-Post` headers still point at the one-click endpoint; the visible links are **additional, not a replacement**. Driver is Gmail/Yahoo bulk-sender deliverability. |
| `D-16` | **OVERRULED in favour of the simpler option - option B: make `/login` a normal, unrouted link that forces a full page load**, so Turnstile's automatic rendering just works. Do **not** call Turnstile's `render()` from an effect. | Supersedes the recommendation (option A, explicit `render()` from the login component effect). Michael's reasons: complexity concern, and the login page should load very fast. **Mechanism VERIFIED:** `leptos_router` 0.8.14 honours `rel="external"`; a plain `<a>` alone is NOT enough because interception is window-level. **WP-55 must also fix three `use_navigate` redirects to `/login`** that no `rel` attribute can cover. Full detail in the `D-16 mechanism` note below. |
| `D-17` | **ACCEPTED**, with a **standing process change** that binds the whole dependency group, not just D-17: the **first** step for any dependency problem is **"upgrade all dependencies to latest and see where we stand"** - the sqlx 0.8/0.9 split may simply resolve. Only if it does not, vendor the `tower-sessions-sqlx-store` (option B). | Michael's strategy is to stay as close to latest as possible so dependencies never go stale. Apply this ordering to WP-64, WP-65, WP-66, WP-67, WP-69, WP-70, WP-71, WP-72 as well: upgrade first, then decide whether the recorded workaround is still needed. |
| `D-18` | **ACCEPTED - trim sentry to explicit features** (backtrace, contexts, panic, tracing/tower as used, native-tls transport), verified with `cargo tree`. | **Standing constraint: it is critical that no Sentry functionality is lost.** The trim must be verified to **preserve current behaviour**, not merely to shrink the dependency tree. Enumerate the sentry features in use before removing any. Preserve the deliberate native-tls transport choice. |
| `D-19` | **ACCEPTED - option A:** `[workspace.dependencies]` **and** `[workspace.package]` **and** `[workspace.lints]` in one migration, early. | Marginal cost inside the same sweep is near zero; workspace lints help every later package. Also resolves the `dp F9` version-pin row in T3-B8. |
| `D-20` | **ACCEPTED - option B: a generic bin crate parameterised over the `Gamer` trait, with thin per-game wrapper bin crates. Explicitly NOT the macro option (A).** | Michael approved it **partly because it avoids macros**. **Standing constraint:** he is wary of custom macros because of maintenance and cognitive cost - **keep any macro surface small and obvious, and pause and discuss if a macro starts getting really complex.** Naming: `game-bin` was the intent, but the repo convention gives **`rust/lib/game_bin`, package `brdgme_game_bin`** - see the `D-20 naming` note below. |
| `D-21` | **ACCEPTED - option A: `serde_yaml_ng`.** | Drop-in API, maintained. JSON (option D) would change a file format ops and users may depend on. bot and lib/game_client move together. |
| `D-22` | **ACCEPTED - port warp -> axum now**, in the same window as WP-06's `http.rs` fixes, so the surface is touched once. | It is one endpoint, but it is the HTTP layer of all 28 game binaries. |
| `D-23` | **ACCEPTED - flip `multiple-versions` to deny AFTER WP-66/67/68 land**, with residual duplicates enumerated in skip/skip-tree, and clear the 4 stale advisory ignores now. | Land WP-69 **last** among the dependency packages so the skip-list starts minimal. |
| `D-24` | **ACCEPTED - option A: accept `combine` 4.6 as a recorded risk**, note it in `deny.toml`, migrate only when the parser is next rewritten. | WP-02 already changes markup enough for one release. No advisory against combine today. |
| `D-25` | **ACCEPTED - option A: port splendor-2 onto `lib/cost`** (add generic `get`/`set`; keep splendor's gold-joker `can_afford` crate-local). | **Constraint: the shared `lib/cost` must have a suitable amount of automated testing as part of the port.** Scope note: D-25 gates only **3 of WP-17's 8 findings** (`b F31`, `ls F39`, `dp F27` - one indivisible consolidation); the other 5 were always implementable. `checklists/T3-B3-splendor-libcost-holdem.md` holds the authoritative row split. |
| `D-38` | **ACCEPTED as recommended, all four sub-items:** (i) implement OneOf offset propagation; (ii) align spec `expected()` impls to typed behaviour and extend the parity tests to cover `expected()`; (iii) adopt UniCase in `suggest`; (iv) skip the deserialized-spec depth guard (specs cross no trust boundary today). | **Standing constraint binding on WP-04 as a whole, not just these four items: keep the parser as straightforward and obvious as possible.** It is complex but critical to the app and must stay reliable and maintainable. Prefer the plainer implementation over the cleverer one at every choice point in WP-04. |
| `D-39` | **ACCEPTED - option A: delete the dead color parse API** (`from_hex` / `from_str`). | Drops `regex` and `lazy_static` workspace-wide and resolves the three-way alias-table divergence by deletion. Git can resurrect it. |
| `D-40` | **ACCEPTED - option B: delete the dead stats machinery** in acquire-1 (`to_brdgme_stats`, `c F12`) and lost-cities-1/-2 (`e F39`, `e F40`), and **split these items out of WP-20/WP-30 into their own package** so they land ahead of the rules review. | They are stats questions, not rules questions. **For the record:** Michael wants to revisit "game specific stats" in future **from a clean slate** - which is exactly why deleting the dead machinery now is right, rather than wiring it into a platform path that does not exist. The split-out package is **WP-81** in `work-packages.md`. |
| `D-35` | **ACCEPTED - keep the park.** Review per game, prioritising **acquire-1, seven-wonders-1 / splendor-2, modern-art-2, red7-1**. Do not lift the park globally. | Those four unblock the most other work. `BLOCKED-ON-USER-RULES-REVIEW` remains stronger than `BLOCKED-ON-DECISION`: it clears only on Michael's per-game sign-off. The five egregious candidates below are now ruled on individually and are the **only** movement out of the park. |
| `a F1` | **FIX NOW, outside the park.** roll-through-the-ages-2 `roll()` re-matches `self.phase` after `keep_skulls()` may have advanced it. | Cross-player state corruption is in no edition: the previous player's `roll()` decrements the **next** player's `remaining_rolls`. The crate's own `test_game_keep_skulls_all_disaster_leadership` asserts the opposite for the `next`-command path, so the fix must adjudicate that test. Rest of WP-12 stays parked. |
| `b F4` | **REMOVED from the egregious list and PARKED** under the rules review. | **Michael's correction, binding: 7 Wonders resources are NOT depleted by trade.** They are printed on cards and both players use them, so there is no competition for a resource and the "asymmetric advantage" framing was **wrong**. **Residual, narrower question - recorded so it is not lost, parked for Michael's review, NOT scheduled:** because players resolve in seat order against live state, player p+1 can trade for a resource card player p **built on that same turn**, which p could not have done in reverse. That is a **simultaneity** question, not a scarcity one. |
| `b F7` | **FIX NOW, outside the park.** seven-wonders-1 must ensure **only one of each physical board can be in play**. | `cities()` lists all 14 A/B entries and `start_game` takes the first `players`, so Rhodes A and Rhodes B can both be dealt. Every printing has 7 boards with one side chosen each; 14 independent boards are physically unreachable. Rest of WP-16 stays parked. |
| `e F30` | **CONDITIONAL - and the condition is SATISFIED, so: FIX NOW.** The rule was to fix the seat-order tie-break only if the correct behaviour is officially described or universally accepted, and to park it if resolving it needed a subjective judgement. **It is described.** | red7-1's own `DATA_DOCS.md` documents the second tie-break - "then by the highest card overall in the palette" - and official Red7 rules agree; the code simply never implements it. **No subjective judgement is required, so this is released from the park.** Evidence and the cause (`leader()` only ever sees the already-filtered winning sets) are in the `e F30 evidence` note below. The **D-29 half - "can an empty winning set win at all" - stays PARKED.** |
| `d F37` | **REJECTED - not a bug.** Do not "fix" this later. | Michael: **this is the accepted way to play.** If only one artist has cards, 2nd and 3rd go to the artists in order from the top. Confirmed by Michael: modern-art-2's `suits()` already returns the suits in canonical top-to-bottom order (Lite Metal top, Krypto bottom), so in the zero-card case Lite Metal is first priority for the placing. Source check confirms `end_round` scans `suits()` in declared order with a strict `>`, so the first suit in that order wins among equal counts - which **is** the correct behaviour. There is no value-board-order-vs-array-index discrepancy. **No follow-up and no fix.** |
| `N-1` | **ACCEPTED** - WP-38 ships with the 15-minute stuck-bot-turn sweep threshold and the 60s `AckKind::Progress` ack-heartbeat cadence. | Tunable config, not load-bearing on the design; revisit from production data. |
| `N-2` | **ACCEPTED** - WP-10 replaces zombie-dice-2's draw-ordered cup with `PubState::cup_counts: Vec<(Colour, usize)>`. | A bot-client-visible API shape change, not a persisted-state change. Any redaction that closes the leak changes the shape; fixed Green/Yellow/Red counts are the cleanest form and no bot can legitimately rely on the leaked order today. |
| `N-3` | **ACCEPTED** - the shared `game_types.player_counts` row uses **newest-non-deprecated-version-wins**, not a union of all versions' counts. | A union would let roster validation accept a player count the actually-selected version cannot run, since new games pick via `find_latest_non_deprecated_game_version`. |
| `N-4` | **ACCEPTED** - the separate `BASIC_STRATEGY.md` / `ADVANCED_STRATEGY.md`, surfaced via `Gamer::basic_strategy` / `advanced_strategy`, **satisfy** `RULES_AUTHORING.md`'s mandatory "Strategy Tips" section. Amend `RULES_AUTHORING.md` accordingly. | **Rationale, which the amendment must state - not merely the permission:** the two files are deliberately separate **so bot difficulty can be tiered.** Every bot gets BASIC to stop it making game-throwing moves; only hard bots also get ADVANCED. **They must not be folded into RULES.md.** Unblocks WP-75. |
| `N-5` | **ACCEPTED** - apply the drafted `docs/BACKLOG.md` item #53 (the parity park). | Michael added: "please ensure docs/BACKLOG.md is correct." **Re-read the live `docs/BACKLOG.md` first** - it is modified in the working tree - and confirm the drafted item is still accurate and **correctly numbered** before declaring it ready. Do not fold into existing item #37; #37 is about verification testing and is downgraded, so folding would hide the park. |
| `N-6` | **ACCEPTED** - apply the drafted 6-rule `## Request-Path Invariants` section to `docs/CODING.md` **as drafted**, at the stated insertion point between `## Rust: Error Handling` and `## Leptos: SSR and Hydration`. | Each of the six rules is the root cause of a critical or major finding; the insertion point was verified against the live file. Source: `CODING-md-amendment-proposal.md`. |

---

## Verification notes

### D-16 mechanism - forcing a full page load for `/login`

**VERIFIED 2026-07-26 by reading the vendored router source. `rel="external"`
works. The ruling is settled, with one gap that WP-55 must also close.**

- Tree versions (`rust/web/Cargo.toml`): `leptos = "0.8.20"`,
  `leptos_router = "0.8.14"` (`Cargo.lock` resolves 0.8.14 exactly).
- **`rel="external"` IS honoured** in this version.
  `leptos_router-0.8.14/src/location/mod.rs` reads the DOM `rel` attribute,
  splits on space/tab, and returns early - letting the browser handle the
  event - if any token is `external` (or if the anchor has `download`). So
  `rel="external"` and `rel="noopener external"` both work.
- **A plain `<a>` is NOT enough on its own.** `leptos_router-0.8.14`
  registers a **window-level** click listener
  (`src/location/history.rs`, `window_event_listener(ev::click, ...)`) and
  walks `composed_path()` for any `HtmlAnchorElement`. It does not care
  whether the anchor came from `<A>` or a literal `<a>`. `rel="external"` is
  required either way.
- **`<A>` has no `rel` prop** (its props are `href`, `target`, `exact`,
  `strict_trailing_slash`, `scroll`, `children`). Two clean options:
  (1) `<A href="/login" attr:rel="external">` - attribute spreading onto `<A>`
  is already proven in this codebase at `rust/web/src/app.rs` (`attr:class`
  on the `/login` link); or (2) a plain
  `<a href="/login" rel="external">`, which is simplest - `<A>`'s only extra
  behaviour is `aria-current` active marking, irrelevant for a login link.
- **Current `/login` links, both `<A>`, both client-side routed today:**
  `rust/web/src/app.rs` (the `index-cta` "Start a game" link) and
  `rust/web/src/components/layout.rs` (the "Login" nav link).
- **GAP that `rel="external"` cannot cover - WP-55 must handle it too.**
  Three navigations to `/login` go through `use_navigate`, which never
  touches an anchor and so is never intercepted by the `rel` check:
  `rust/web/src/components/layout.rs` (post-logout),
  `rust/web/src/settings.rs` (anonymous redirect), and
  `rust/web/src/admin.rs` (anonymous redirect). These need a hard navigation
  (a location assignment) rather than `use_navigate`, or Turnstile will still
  fail to render when a user reaches `/login` by those paths.
- Turnstile context confirmed: the `api.js` `<script async defer>` lives in
  the shell head in `rust/web/src/app.rs`, and the
  `<div class="cf-turnstile" ...>` widget is in the same file.

### D-20 naming - the generic game bin crate

**VERIFIED 2026-07-26. Concrete name: `rust/lib/game_bin`, with
`[package] name = "brdgme_game_bin"`. Do NOT use `game-bin` /
`brdgme-game-bin`.**

The convention, read from the workspace members list and the crate manifests:
shared crates under `lib/` and `tools/` use **snake_case directory names** and
package name **`brdgme_<snake_dir>`** - consistent across all 10
(`lib/cmd` -> `brdgme_cmd`, `lib/game_client` -> `brdgme_game_client`,
`tools/fuzz` -> `brdgme_fuzz`, ...). Hyphens are the **game-crate**
convention, where the directory name equals the package name with no prefix
(`game/red7-1` -> `red7-1`). `brdgme-operator` is the single hyphenated
outlier and is not under `lib/`. `lib/game_client` -> `brdgme_game_client` is
the direct precedent for a two-word snake name.

Implementation note for WP-73: each game crate today carries **4 `[[bin]]`
targets inside itself** at `src/bin/<snake_name>_{cli,fuzz,http,repl}.rs`,
each a 3-10 line `Gamer`-parameterised call (e.g. `http::serve::<Game>(addr)`).
The `[[bin]]` machinery lives in the game crates, not in separate bin crates,
so "thin per-game wrapper bin crates" is a structural change, not just a move.

### e F30 evidence - is the red7 tie-break officially described?

**VERIFIED 2026-07-26: NOT subjective. The condition in the `e F30` ruling is
SATISFIED, so `e F30`'s seat-order half is FIX NOW.**

red7-1's own `DATA_DOCS.md` states verbatim: "Ties within a rule are broken by
the highest card in the winning set, then by the highest card overall in the
palette." That documented **second** tie-break is exactly the all-empty-
winning-set case, and it is simply **not implemented**. Official Red7 rules
agree (highest card in palette as the ultimate tie-break, card value = number
then colour - which `Card::rank_key` in `rust/game/red7-1/src/card.rs` already
encodes as `(rank, suit ordinal)`). `RULES.md` is silent, but the crate's own
data doc plus official rules are enough; **no subjective judgement is needed.**

Cause: `leader()` in `rust/game/red7-1/src/card.rs` receives the **already-
filtered winning sets** (`rust/game/red7-1/src/lib.rs` pushes
`rule_fn(&self.palettes[p])`), so the full palette is unreachable from
`leader()`. When all are empty, every `len()` is 0 and every `rank_key()` max
is `(0,0)`, the strict `>` never fires, and `leader_idx` stays 0 - lowest
surviving seat wins, contradicting `DATA_DOCS.md`. Reachable via Green
(`most_even_cards`) or Violet (`most_cards_below_4`) with all-odd / all-rank-4+
palettes.

The fix is to fall through to the **full palette's** `rank_key()` max, which
requires plumbing the unfiltered palette into `leader()`. The separate D-29
question ("can an empty winning set win at all") **stays parked**.

### d F37 - confirmed rejected, no follow-up

Recorded for completeness. `end_round` in
`rust/game/modern-art-2/src/lib.rs` scans `suits()` with a strict `>`, so among
equal counts the **first suit in `suits()` order** wins; `suits()` in
`rust/game/modern-art-2/src/card.rs` returns the fixed enum-declaration order
`[LiteMetal, Yoko, ChristineP, KarlGitter, Krypto]`. Michael confirms that
order **is** the canonical top-to-bottom value-board order (Lite Metal top,
Krypto bottom), so iterating `suits()` in declared order is the correct
behaviour and the earlier "value-board order vs array index" caveat is void.
**No fix, no follow-up.**

---

## Standing constraints extracted from these rulings

These bind implementers beyond the single row that produced them:

1. **Dependencies (from D-17):** for any dependency problem, upgrade
   everything to latest FIRST and re-assess. Only then take the recorded
   workaround. Applies across WP-64..WP-73.
2. **Macros (from D-20):** keep any macro surface small and obvious. Pause
   and discuss if a macro starts getting really complex.
3. **Parser (from D-38):** WP-04 keeps the parser as straightforward and
   obvious as possible. It is complex but critical; reliability and
   maintainability beat cleverness.
4. **Sentry (from D-18):** no Sentry functionality may be lost to the feature
   trim. Verify behaviour preservation, not just tree size.
5. **`lib/cost` (from D-25):** the shared crate must gain a suitable amount of
   automated testing as part of the port.
6. **Parity park (from D-35):** still in force per game. Only `a F1` and
   `b F7` are released from it, plus `e F30` conditionally.
