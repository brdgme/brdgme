# Remediation backlog - prioritized work-package ordering

Ordering axes: severity (criticals and account-takeover/data-corruption
first), user impact, unblocking value, effort. Decision-blocked packages
are listed in the phase where they should land, tagged with the D-item
that unblocks them (see decisions-needed.md). Packages within a phase are
in suggested execution order. WP details: work-packages.md.

## Phase 0 - decisions batch 1 (no code)

Get answers to the security/integrity decisions so Phase 1 is never
starved: D-1 (From-auth), D-3/D-4 (undo-ratings), D-5 (bot wedge), D-8
(bot slots), D-6 (visibility), D-12 (fail-open), D-2 (delivery
semantics). One short session with Michael answers all of these from
decisions-needed.md.

## Phase 1 - security and data-corruption criticals

1. WP-44 proposals integrity + email_token leak - READY. The leaked
   email_token is the credential that composes with forgeable From into
   takeover; dropping it from the payload is small and immediate.
2. WP-56 email From-auth redesign - D-1. Kills both remaining criticals
   (account takeover).
3. WP-01 char/byte panic elimination - READY. 5 criticals; request-
   reachable panics from ordinary input (iOS NBSP is a live trigger),
   server- and WASM-side.
4. WP-40 undo/concede TOCTOU + ratings - D-3. Permanent rating
   corruption (critical) + 6 majors, one db.rs root cause.
5. WP-14 alhambra core fixes - READY. Critical money-duplication exploit
   from crafted input.
6. WP-25 modern-art liveness - READY. Critical infinite busy-loop
   (hang + unbounded log growth) legally reachable in round 4.
7. WP-36 crypto/deploy hardening - READY. Missing Secure cookie flag in
   prod; small package.
8. WP-45 bot-slot validation - D-8. Two majors; stops new wedged games
   being creatable while Phase 2 builds recovery.

Rationale: everything user-abusable or data-corrupting, mostly small
packages; 5 of 8 are READY today.

## Phase 2 - platform correctness majors + early unblockers

9. WP-64 workspace-deps migration - D-19. Deliberately early: touches
   all 40 manifests while few other branches are open, and turns every
   later version bump into a one-line edit.
10. WP-68 term_size replacement - READY. RUSTSEC advisory; trivial.
11. WP-39 bot consumer supervision - READY. Silent permanent bot outage
    class; independent of the D-5 design.
12. WP-38 bot-turn wedge recovery - D-5. The remaining wedge modes
    (UserError ack, retry exhaustion, bot rename deadlock).
13. WP-46 sweep delivery semantics - D-2/D-11. Dropped turn commands and
    double-sends.
14. WP-57 inbound webhook delivery - D-2. Same semantics decision;
    permanently-dropped inbound commands.
15. WP-47 game_visibility gates - D-6. Privacy model wired to reads.
16. WP-42 websocket pass - D-13. Do with/after WP-47 (same visibility
    predicate).
17. WP-34 auth races/session mechanical - READY.
18. WP-35 auth edges + fail-open - D-12/D-14.
19. WP-49 rules/game-info pages - D-6. F67 (wrong rules version served)
    is high user impact and can lead the package.
20. WP-06 lib cmd tools/http - READY. Contains the one production warp
    handler panic (ls F19).
21. WP-07 game_client/rand_bot - READY. Unbounded operator hang (ls F31).

## Phase 3 - game correctness majors

22. WP-13 starship-catan - READY. 5 majors, three reachable by legal play.
23. WP-15 seven-wonders mechanical - READY. Soft-lock + scoring majors.
24. WP-22 lords-of-vegas - READY. Render underflow in ordinary 5-6p play.
25. WP-23 jaipur - READY. Scoring defect (bonus tokens).
26. WP-28 lost-cities pair - READY. 3p stats major; request-reachable
    panics also covered by WP-09 if D-36 lands first - coordinate.
27. WP-19 acquire - READY. 6p never offered; dummy die bug.
28. WP-21 cathedral + sushizock - READY. Traffic-driven memory leak;
    overflow panic.
29. WP-09 deserialized-state trust hardening - D-36. Systemic panic
    class; the requester-boundary fix protects every crate at once.
30. WP-10 pub_state redaction - D-33. Hidden-info leaks (2 majors).
31. WP-03 lib-game parser mechanical - READY.
32. WP-02 markup robustness - D-37.
33. WP-41 db.rs quality pass - READY (contains the untested-fns major).
34. WP-37 admin.rs pass - READY.
35. WP-59 inbound processing quality - READY.
36. WP-58 unsubscribe RFC 8058 - D-10 (deliverability-relevant, do
    before any email-volume growth).

## Phase 4 - rules adjudication batch (after decisions batch 2)

Answer D-35 first, then D-26..D-34 in one sitting; implement per crate:

37. WP-26 batch-d rules (modern-art scoring cluster, jaipur, sushi-go,
    LoV counts) - D-26/D-30/D-32.
38. WP-16 batch-b rules (seven-wonders, alhambra, splendor tie-break) -
    D-27/D-28.
39. WP-30 batch-e rules/stats (red7 empty-set, lost-cities stats) -
    D-29/D-40.
40. WP-20 batch-c rules/edition (acquire edition trio, holdem cap,
    acquire stats) - D-30/D-31/D-40.
41. WP-11 batch-f port parity - D-35/D-30.
42. WP-12 rtta-2 - D-34.
43. WP-17 splendor + lib/cost - D-25.
44. WP-29 red7 cleanup - READY, but sequence after WP-30 (doc rewrites
    depend on the D-29 outcome).

## Phase 5 - quality, consistency, cleanup

45. WP-54 frontend UX error handling - READY (1 destructive-action major).
46. WP-55 Turnstile SPA rendering - D-16 (login-blocking for SPA
    navigators; can be pulled into Phase 2 if users report it).
47. WP-51 invite-mailer/notify dedup - READY.
48. WP-60 outbound tokens/metrics/render - READY.
49. WP-52 stats/query perf pass - READY.
50. WP-53 domain misc server fns - READY.
51. WP-50 email canonicalization - D-9.
52. WP-48 export/import - D-7.
53. WP-61 bot service quality - READY.
54. WP-62 operator - READY (finalizer race major).
55. WP-63 fuzz tool - READY (hang major, dev-tooling only).
56. WP-08 epilogue-dup sweep - READY.
57. WP-27 love-letter + age-of-war - READY.
58. WP-24 sushi-go - READY.
59. WP-18 texas-holdem - READY.
60. WP-31 zombie-dice + battleship - READY.
61. WP-32 for-sale + category-5 - READY.
62. WP-33 small-crate cleanup - READY.
63. WP-04 lib-game parser design items - D-38.
64. WP-05 lib color - D-39.
65. WP-43 web cargo deps - READY.

## Phase 6 - dependency structure (sequenced within itself)

66. WP-66 sqlx unification - D-17.
67. WP-67 sentry trim - D-18 (before lock re-audit).
68. WP-70 serde_yaml migration - D-21.
69. WP-71 warp->axum - D-22 (pair with WP-06 if not already done).
70. WP-73 game-bins consolidation - D-20 (after WP-64; touches all game
    manifests).
71. WP-72 combine posture - D-24 (likely a deny.toml note only).
72. WP-65 workspace hygiene - READY (after WP-64).
73. WP-69 deny.toml hardening - D-23. LAST: flip warn->deny once
    WP-66/67/68 have shrunk the duplicate set.

## Phase 7 - documentation follow-ups (lowest priority)

Filed at spec time, not by the review; zero finding IDs (see
work-packages.md's "Documentation packages filed at spec time"). Both touch
`rust/game/red7-1/RULES.md`, so both must land after WP-29 Task 5 and after
WP-30 (D-29 may change how elimination is described). Either can be folded
opportunistically into whichever of those lands last.

74. WP-74 red7-1 empty-hand-elimination rules doc - READY. One missing
    sentence; a bot reading only RULES.md cannot predict its own elimination.
75. WP-75 red7-1 RULES.md RULES_AUTHORING.md compliance - READY but NOT
    spec-writable from source alone: needs a live render capture and a ruling
    on whether the shipped strategy docs satisfy the Strategy Tips section.
    Do after WP-74.

## Notes

- Phases 3-5 game packages are independent of each other; they can be
  freely reordered or parallelized across sessions.
- WP-09 (state-trust) intersects the per-crate game packages: if D-36
  picks the requester-boundary fix, land WP-09 before the bulk of
  Phase 3's per-crate work so those packages can drop their defensive
  duplicates; if per-crate defensive is chosen, fold WP-09's items into
  the per-crate packages instead.
- Every fix recommendation must be re-validated at spec time
  (verification proved several original recommendations wrong - see
  work-packages.md notes and planning/raw/ grouping notes).
- Decision batches: batch 1 (Phase 0) = D-1..D-8 core; batch 2 (before
  Phase 4) = D-35 + D-26..D-34; batch 3 (any time) = D-9..D-25,
  D-36..D-40.
