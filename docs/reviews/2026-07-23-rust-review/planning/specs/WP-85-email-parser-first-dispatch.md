# WP-85 - Email dispatch: game parser first, platform commands as fallback

> # Status: DEFERRED - BLOCKED ON MICHAEL. An executor must NOT pick this up.
>
> This is a sketch of a future change, not an execution script. It is blocked on a
> decision Michael has deliberately not made yet (see **Blocker**). Do not implement,
> do not "just do the easy half", do not invent the missing input.

**Findings: 0.** A carve-out, not a new body of findings - exactly as WP-83 was.

## Provenance

Carved out of **WP-59 Task 14** on 2026-07-26 per Michael's ruling:

> "WP-59 Task 14 sounds like a risk, let's pull it out to a separate item if we can."

Task 14 was written as a documentation edit, but the work it implies is a **behaviour
change** in `dispatch_email_command`. A behaviour change riding inside a documentation
package was the risk; the carve-out removes that coupling. Task 14's body has been
deleted from `specs/WP-59-inbound-processing-quality.md` (heading kept, `- CARVED OUT`).

## Blocker - the single reason this is deferred

The **escape-hatch verb set** - the small hard-reserved set (`help` and equivalents)
that must win *even on the game path* - is undecided **by deliberate choice**:

> "can we just defer that work? No games use those verbs yet, and I think I'd like the
> current version of brdgme in place a bit longer so I can get a feel for if and how we
> want to do this in the future."

**Its membership must NOT be invented.** Deciding it is a hard prerequisite. This spec deliberately does not design it and proposes no candidate list.

## Authority

**D-15**, ANSWERED 2026-07-26 in `planning/decisions-ANSWERED.md`. Operative sentence:

> "Do not hardcode a reserved-verb list. On game-scoped messages, try the game command
> parser FIRST; platform commands are the FALLBACK when the game parser fails."

One carve-out: a small hard-reserved set of escape-hatch verbs always wins, even on the game path.

## Current code (verified by reading, 2026-07-26)

`rust/web/src/email/commands.rs` - under concurrent edit by another agent, **expect
drift**; navigate by symbol, never by line number.

`pub async fn dispatch_email_command(ctx: &EmailCommandCtx<'_>, line: &str) -> Result<CommandReply, CommandError>`
(approximate line 1204 - verify). Confirmed shape:

- Splits `line.trim()` on the first space into `(verb, arg)`; ASCII-lowercases the verb.
- **Platform-first, game-last.** `match verb_lower.as_str()` hardcodes `concede`, `end`,
  `undo`, `restart`, `rules`, `help | commands`, `new`, `bump`, `list`; `_ => {}` falls
  through.
- Then two further platform gates: `subscribe_toggle(&verb_lower)`
  (`subscribe`/`unsubscribe`) and `dispatch_settings_command(ctx, trimmed)` (verbs from
  `settings_verb`: `name`, `colors|colours`, `theme`, `emails`, `settings`).
- Only if all miss does it call `crate::game::execute_command(...)` with the whole
  `trimmed` line.
- **18 verbs total** (10 match arms incl. `commands`, + 2 subscribe, + 6 settings incl.
  `colours`).
- There is currently **no** source comment asserting the reservation.

## The change, when it happens

Invert `dispatch_email_command`: on **game-scoped** messages try
`crate::game::execute_command` FIRST, and fall back to the platform command dispatch
only when the game parser fails. The escape-hatch set is checked ahead of everything.
**Non-game-scoped (standalone) messages are unaffected** - platform commands are all
there is on that path.

### Open sub-question - UNKNOWN today, do not guess

How is "the game parser did not recognise this line" (a parse miss, which must fall back
to platform commands) distinguished from "the game parser recognised it and the user got
it wrong" (a legitimate user error, which must be reported, not silently re-dispatched)?

Read: `dispatch_email_command` maps `crate::game::ExecuteCommandError::UserError(msg)` /
`::Conflict` / `::Other(e)` onto `CommandError`. `UserError` carries only a `String`, so
on today's surface a parse miss and a user error are **indistinguishable at this call
site**. Whether `execute_command` itself can distinguish them is **UNKNOWN** - not read.
The implementer must settle this against the real `execute_command` error type before
writing any code.

## The 18-verb list is DELETED, not documented

`docs/authoring/COMMANDS.md` flips from "avoid these verbs" to "your parser is tried
first; only these few escape-hatch verbs are off-limits". WP-59 Task 14's original
option-A COMMANDS.md text was deleted in the carve-out and lives only in git history -
**do not resurrect it.**

## Fixes the acquire-1 / starship-catan-1 `end` collision

Both expose `end` as a top-level move:
`rust/game/acquire-1/src/command.rs` (`end_parser`, approximate 192-197) and
`rust/game/starship-catan-1/src/command.rs` ("end the flight early", approximate
316-319). Both approximate - verify. Today the dispatcher's `end` arm intercepts first,
so **neither move is playable by email**. Parser-first makes them playable with **no
game-crate change**.

## CONSEQUENCE - WP-59 text becomes FALSE when WP-85 lands

Once WP-85 lands, WP-59's **"Known collisions"** list and its **"there is deliberately no
escape prefix ... the reservation is absolute"** paragraph are **false statements**, as is
any COMMANDS.md text derived from them. They live in the (now carved-out) Task 14 region
of `specs/WP-59-inbound-processing-quality.md`. Whoever lands WP-85 **must** check and fix
them. The Task 14 body was already deleted in the carve-out, so **verify what actually
survives at landing time** rather than assuming these strings are still present.

## Navigation rule

Navigate by symbol: `dispatch_email_command`, `subscribe_toggle`, `settings_verb`,
`dispatch_settings_command`, `crate::game::execute_command`. **Not** by line number - all
line numbers here are approximate hints. Read the named function; if it does not match
this spec, **STOP and report** rather than improvising.

## Prerequisites

1. **Michael decides the escape-hatch verb set.** Blocking. Not to be invented.
2. **The parse-miss-vs-user-error question is settled** against the real
   `execute_command` error type.
