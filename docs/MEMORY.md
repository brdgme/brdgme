# Session Memory

## Project Overview
brdgme: lo-fi multiplayer board gaming platform, 10+ years old, real users.
Play via web or email, ASCII rendering, text commands, bot support.
All open source, always.

## Repo Structure
- `rust/web` - new Axum+Leptos monolith (nearly complete, `leptos` branch)
- `rust/api` - old Rocket API (to be deleted post-cutover)
- `web` - old React/Redux/Webpack frontend (to be deleted)
- `websocket` - old Node.js WebSocket service (to be deleted)
- `brdgme-go` - Go game implementations (~20 games)
- `rust/game` - Rust game implementations (Acquire, Lords of Vegas, Lost Cities x2)
- `rust/lib` - shared Rust libraries (brdgme_cmd, brdgme_game, brdgme_color, brdgme_markup)
- `k8s/` - Kubernetes manifests
- `docs/` - VISION.md, ARCHITECTURE.md, PLAN.md, REVIEW.md (this file)

## Docs Structure (created this session)
- `docs/VISION.md` - timeless goals, no status
- `docs/ARCHITECTURE.md` - target arch + stable game JSON contract + Mermaid diagrams
- `docs/PLAN.md` - Axum/Leptos migration phases + implementation log (merged from root PLAN.md + IMPLEMENTATION.md)
- `docs/REVIEW.md` - comprehensive code review of rust/web (in progress, stopped mid-review)
- `docs/adr/` - empty, ready for ADRs
- Deleted: root PLAN.md, IMPLEMENTATION.md, ARCHITECTURE.md, k8s/README.md, all rust/web/*.md noise files

## Target Architecture (agreed)
- **Platform**: DigitalOcean Kubernetes, Sydney (SYD1), ~$63/month baseline
- **CNI**: Cilium (default on DOKS)
- **Always-on core**: Axum+Leptos monolith (rust/web), multiple replicas
- **WebSocket fan-out**: NATS Core in-cluster (replaces tokio::sync::broadcast for multi-replica)
- **Game services**: Knative Serving (scale-to-zero), existing HTTP JSON contract unchanged
- **Operator**: Custom Rust operator (kube-rs) watching GameType CRDs
  - Creates Knative Service per CRD
  - Upserts game_versions in PostgreSQL
  - Soft-delete (is_available=false) via Finalizers
  - Startup reconciliation sync as safety net
- **Database**: PostgreSQL (game_versions has unique index on (name,version), is_available flag)
- **Ingress**: Cilium Gateway API + single load balancer
- **No Redis** (replaced by NATS), **No Node.js**, **No Rocket**
- NATS Core→JetStream upgrade path: single config flag, zero code change, needs volume for persistence

## Long-term (out of scope now)
- **Email**: third-party provider (Mailgun/Postmark), inbound via webhook→Knative Service
- **Bots**: LLM-based, Knative-invoked. Start with external API (Groq), target Ollama in-cluster CPU inference. Constrained generation for valid commands.
- **No self-hosted SMTP** (had deliverability issues in old Go version)
- **No Kafka/RabbitMQ** (NATS if ever needed)

## Planning Convention
- `docs/` for stable narrative docs
- GitHub Issues for active tasks
- No GitHub Projects (solo project)

## REVIEW.md Status (COMPLETE)
Full parity review done this session covering rust/api, web (React), and websocket (Node.js) vs rust/web.
37 items documented in REVIEW.md (8 blockers, 29 gaps).
See docs/REVIEW.md for the complete list.

## Key Findings from Review

### Blockers (must fix before cutover)
1. `create_game` + `play_command` in game/server.rs use `Uuid::nil()` - unauthenticated
2. Login UI (app.rs) not wired to server functions - web login non-functional
3. Confirmation token exposed in login response JSON - security issue
4. Session store is MemoryStore - lost on restart, doesn't work across replicas
5. `with_secure(false)` on cookies - must be env-driven
6. No graceful SIGTERM shutdown in main.rs
7. Turn enforcement missing in play_command handler

### Missing Features vs rust/api (COMPLETE - see REVIEW.md)
- `POST /game/{id}/undo`, `mark_read`, `concede`, `restart` - all missing
- `GET /init` - no equivalent (game type listing, active games, user bootstrap)
- `GamePlayer` model missing: last_turn_at, is_eliminated, is_read, points, undo_game_state, rating_change
- update_game_command_success doesn't write: is_turn_at, last_turn_at, is_eliminated, undo_game_state, points
- find_game_extended errors if player has no game_type_users row (migration risk)
- validate_session_token has no expiry check (tokens are permanent)
- New-game creation UI entirely missing (GamesPage is stub)
- Game logs stub, no undo/restart/concede UI, no clickable suggestions
- Auth: old = Bearer token header, new = session cookie (breaking change)
- Login confirmation: old = 6-digit numeric, new = UUID (better, but exposed in response body)

### Known Gaps (non-blocking)
- GameLogs component is a stub (no real logs rendered)
- Points not persisted (_points unused in update_game_command_success)
- reqwest::Client created per-request (should be shared/pooled)
- N+1 query in find_active_games_for_user
- Duplicate game command logic in server.rs vs server_fns.rs
- WebSocket no reconnection logic
- NaiveDateTime should be DateTime<Utc> throughout models
- Dead code: New* model structs, chat.rs, friends.rs, PublicGameType alias, SESSION_AUTH_TOKEN_KEY, db::AppState
- DashboardPage and GamesPage are stubs
- Logout has no redirect/feedback
- GameCommandInput clears input before server confirms success
- submit_action result not observed (errors silently dropped)
- db::AppState in db.rs is a dead duplicate of state::AppState

## Next Steps Needed (priority order)
1. Fix blockers 1-8 (existing) + 21-24 (new) before cutover
2. Expand GamePlayer model to include missing DB fields (item 21)
3. Fix update_game_command_success to write all fields (item 22)
4. Fix find_game_extended to gracefully handle missing game_type_users (item 23)
5. Add token expiry check to validate_session_token (item 24)
6. Implement undo, mark_read, concede, restart endpoints (items 25-28)
7. Add public game version server function for new-game form (item 29)
8. Build new-game creation UI (item 30)
9. Implement game log rendering (item 31)
10. Wire undo/concede/restart actions in GameMeta (item 32)
11. Replace MemoryStore with tower-sessions-sqlx-store
12. NATS integration to replace tokio::sync::broadcast
13. Knative + operator work (post-cutover)
