# Bots are referenced by name

**DECIDED 2026-07-25, REFINED 2026-07-26:** games reference bots BY NAME, not
by id. A dangling bot name is a supported no-op state, not an error that
wedges a game. Bot slots are validated on write and tolerated on read.

## Context

Bot players are stored as a bot name. Renaming or swapping a bot by name is a
product capability, so converting the reference to a stable bot id (via a
migration) was considered and rejected. See `docs/decisions/BOT_EFFICACY.md`
for the bot configuration model (the `bots` table, arbitrary `bot_name`
values, DB-backed config).

## Decision 1 - bots stay referenced by name

Games keep referencing bots by name; there is no bot-id migration. Swapping or
renaming a bot by name is a supported product capability.

## Decision 2 - a dangling bot name is a supported no-op

A bot player name that resolves to nothing - deleted, renamed away, or
disabled - is an explicitly SUPPORTED state. It no-ops rather than wedging the
game: the game does not wedge, the message is acked, and the condition is
surfaced (an admin-page warning lists dangling bot player names) rather than
retried forever. "All bots disabled" is a valid intentional configuration and
must not trip alerts or blocking validation.

## Decision 3 - validate on write, tolerate on read

Validate bot slots at all four creation entry points (`create_proposal`,
`add_proposal_player`, `restart_core`, email `new`) AND at game start, so a
typo or a bot that does not exist right now gets immediate feedback. A name
that goes missing or is disabled LATER - after a game has started - must not
wedge the game and must not be rejected at turn time; it falls into the
dangling-name no-op path plus the admin warning.

## Decision 4 - restart resolves a deprecated bot to the latest version

On restart, resolve a deprecated bot to the LATEST NON-DEPRECATED version of
that bot and start the game with it. This is an active resolution, not a
carve-out: the restart path does not fall into the dangling-name no-op, and it
is not exempt from write-validation. The no-op-plus-admin-warning path remains
correct only for a name that goes missing or is disabled AFTER a game has
started.
