# 24: Game Invites - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.
>
> Extracted 2026-07-08 from `docs/plan/24-game-invites.md`. Task granularity is
> work-package level; run superpowers:writing-plans against the paired spec
> before execution if bite-sized steps are needed.

**Spec:** `docs/superpowers/specs/2026-07-04-24-game-invites-design.md`

## Tasks

Core (no email dependency):

- [ ] Migration: `game_proposals`, `game_proposal_players` (+ indexes on
      `user_id, response` for the dashboard query and on
      `proposal_id`).
- [ ] Server fns: `create_proposal` (replacing direct creation for games
      with human invitees; direct path kept for solo-vs-bots),
      `respond_to_invite`, `replace_slot_with_bot`, `remove_slot`,
      `cancel_proposal`, `start_proposal` (early-start policies +
      `player_counts` validation + auto-start on last accept).
- [ ] `restart_game` rewired to create a proposal (roster + bots carried;
      close or supersede the bugs.md bot-restart item).
- [ ] Dashboard UI: "Invites" section (invites received: accept/decline;
      proposals sent: response status per slot, owner actions, start
      early, cancel). New-game form submits a proposal.
- [ ] WS: publish a skinny signal on proposal changes so open dashboards
      update live (subject/channel naming consistent with whatever
      Phase 17 has done by then - `game.{id}` equivalent for proposals).
- [ ] Tests (Phase 11 patterns): full lifecycle (create → mixed
      accept/decline → replace/remove → start), auto-start on last
      accept, early-start drop and bot-replace policies,
      `player_counts` validation rejects invalid rosters, cancel
      notifications, restart-proposal carries bots, non-owner actions
      rejected, double-response rejected.

Email (after 22b):

- [ ] Invite/decline/cancel/start emails via `email_render`; `i-` token
      routing in the webhook; `accept`/`decline` reply commands.
- [ ] Unsubscribed-invitee warning (creation response + proposal UI
      badge).
- [ ] Reminder + expiry in the 22c sweep task (`INVITE_REMINDER_AFTER`,
      `INVITE_EXPIRE_AFTER`).
- [ ] Tests: email-reply accept/decline (auth rejection cases as in 22b),
      reminder/expiry boundaries, no email to unsubscribed invitees.
