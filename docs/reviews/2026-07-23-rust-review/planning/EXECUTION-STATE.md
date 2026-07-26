# EXECUTION STATE - live tracker (Orchestrator-owned)

Tree started clean at 37118d3. 13 pre-landed: WP-01, 03, 06, 13, 14, 15, 21, 25, 36, 37, 39, 41, 44.

## Landed this session
| WP | Commit | Notes |
|---|---|---|
| WP-82 | 4d31f6e | db.rs module split. Pure move. All verification green. |
| WP-56 (partial) | da1ea24 | Tasks 1,3,4,5,6 landed. Task 2 (SPF/DKIM) PARKED - see below. Migration used: 023. Follow-up details in Lead report: touches email/inbound.rs + resend_webhook. |
| WP-59 | f56ff37 | 12 tasks. Dropped dead: Task 10, Task 11 delete_login_confirmation (WP-56), Task 14 (WP-85 deferred per D-15). classify_server_fn_error exists (private in email/commands.rs - WP-40 may need pub(crate)). fetch_inbound_text(state, email_id) -> Option<String> ready for WP-57 widening. Unsubscribe untouched for WP-58. |

## Planned sequence (from EXECUTION-README s2, constraints applied)
1. ~~WP-56~~ done (partial; Task 2 parked)
2. ~~WP-59~~ done
3. WP-40  <- NEXT (WP-59 now landed; classify_server_fn_error may need visibility bump)
4. WP-45
5. WP-79 (no spec; after WP-40+WP-45)
6. WP-64
7. WP-68
8. WP-38
9. WP-46
10. WP-57 (after WP-59)
11. WP-47
12. WP-42 (predicate work ONLY; D-44 SSE, D-45 no id:, D-48/49 two streams)
13. WP-84 (end of realtime chain)
14. WP-34 (migr: re-ls)
15. WP-35
16. WP-49
17. WP-07
18. WP-83 (parity carve-outs a F1, b F7, e F30 seat-order; independent)
19. WP-09a (fold in WP-80; D-36 before bulk of Phase 3; do NOT fix WP-28's deliberate panic)
20. WP-09b
21. WP-81 (before WP-19; D-40)
22. WP-22
23. WP-23
24. WP-28 (leave self.hands[player] panic in Task 3)
25. WP-19 (drop c F11 / Task 5 per WP-81)
26. WP-10 (N-2)
27. WP-02
28. WP-58 (migr: re-ls; after WP-59+WP-56 - both satisfied)
29. WP-17 (D-25: only 3 of 8 findings gated; lib/cost must gain tests)
30. WP-29 (only if its own spec satisfies the WP-30 ordering note)
31. WP-54
32. WP-55 (rebase onto WP-54's arm, keep WP-37's "/" bounce)
33. WP-51
34. WP-76 (no spec; after WP-51 Task 1; 5-line change; NOT into WP-59/WP-40)
35. WP-60
36. WP-52
37. WP-53
38. WP-50 (migr: re-ls)
39. WP-48
40. WP-61
41. WP-62 (bo F25: confirm k8s-openapi feature flag at fix time)
42. WP-63
43. WP-08
44. WP-27
45. WP-24
46. WP-18
47. WP-31
48. WP-32
49. WP-33
50. WP-04
51. WP-05
52. WP-43
53. WP-77 (no spec; any time)
54. WP-66
55. WP-67
56. WP-70
57. WP-71
58. WP-73 (D-43 REVERSED: keep 27 _fuzz bins, ship 3 entry points)
59. WP-72 (content lives in WP-69 spec; D-24: accept combine 4.6 in deny.toml)
60. WP-65
61. WP-69 LAST (D-23: flip multiple-versions to deny only after 66/67/68)
62. Phase 7 WP-74, WP-75: EFFECTIVELY BLOCKED (WP-30 parked) - do not force; report to user
63. Tier 3 checklists T3-B1..B8

## Parked / skip
- WP-11, 12, 16, 20, 26, 30 (BLOCKED-ON-USER-RULES-REVIEW)
- WP-78 superseded by WP-82; WP-85 deferred
- D-26..D-32, D-34 unruled (except released carve-outs) - never invent rulings
- b F4 re-parked; d F37 REJECTED not-a-bug - never reopen
- Never change gameplay / correct a RULES.md without Michael sign-off

## Migration numbering
Highest on disk at start: 022. WP-34, WP-50, WP-56, WP-58 all add migrations.
First to land gets 023; rest renumber. Re-ls rust/web/migrations/ immediately before writing.

## Needs user input (parked items)
- **WP-56 Task 2 (SPF/DKIM inbound auth classification):** Resend's `email.received` webhook carries NO auth verdict field; verdicts only exist as raw headers in the already-fetched raw MIME. Spec mandates STOP before writing a fallback. NEED: the trusted authserv-id / matching rule for identifying Resend's receiving-MTA `Authentication-Results` header as topmost-trusted (a sender can inject lower ones). Once ruled, implement `classify_inbound_auth` gating in `resend_webhook` after body fetch (spec Task 2). Also note Phase 7 (WP-74/75) is effectively blocked by parked WP-30 - needs a call at the end.
