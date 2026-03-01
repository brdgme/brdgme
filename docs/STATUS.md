# Current Work Status

## Phase 5.5: COMPLETE

All items done:
- Kind cluster + Cilium + Knative Serving
- Tilt dev environment (hybrid + full-cluster modes)
- GitHub Actions CI (`.github/workflows/ci.yml`)
- SQLx offline metadata (`rust/.sqlx/`)
- `k8s/prod/app/kustomization.yaml` updated with all GHCR image mappings
- `build-legacy` job added to CI (web-legacy, websocket, api)
- `skaffold.yaml`, `.travis.yml`, `rust/api/.travis.yml` deleted

## Active Task: Phase 5.6 - Pre-Cutover Fixes

See `docs/PLAN.md` Phase 5.6 for the full list of 13 blockers and parity gaps.

### Recommended order for blockers

1. **Persistent session store** - `tower-sessions-sqlx-store` replaces `MemoryStore`.
   Requires new migration + crate addition. Blocks all auth testing.

2. **Login UI wired to server functions** - `on_email_submit` / `on_code_submit` in
   `app.rs` not connected to `Login` / `ConfirmLogin` server functions.

3. **Confirmation token not exposed in response** - Remove token from login response body.

4. **`with_secure` env-driven** - Read from env, not hardcoded `false`.

5. **Token expiry check** - Add 30-day expiry to `validate_session_token`.

6. **Email sending** - SMTP integration in `auth/server.rs`. SMTP service already in cluster.

7. **Auth in Axum handlers** - Replace `Uuid::nil()` with session user ID in
   `create_game` and `play_command`.

8. **Authenticate `GET /api/game/{id}`**.

9. **Turn enforcement** - Reject commands when not the authenticated player's turn.

10. **`GamePlayer` model missing fields** - Add `last_turn_at`, `is_eliminated`,
    `is_read`, `points`, `undo_game_state`, `rating_change`.

11. **`update_game_command_success` writes all fields**.

12. **`find_game_extended` handles missing `game_type_users` row** - LEFT JOIN.

13. **Graceful SIGTERM shutdown** - `axum::serve(...).with_graceful_shutdown(...)`.
